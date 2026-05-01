use crate::{
    arch::x86_64::reserved,
    memory::{PhysicalAddress, PhysicalFrame},
};

pub const PAGE_SIZE: u64 = 4096;
const ENTRY_COUNT: usize = 512;
const IDENTITY_MAP_LIMIT: u64 = 0x0020_0000;

const PRESENT: u64 = 1 << 0;
const WRITABLE: u64 = 1 << 1;
const WRITE_THROUGH: u64 = 1 << 3;
const CACHE_DISABLE: u64 = 1 << 4;

/// One runtime scratch page inside the existing higher-half stack window.
///
/// Bootstrap code already maps the surrounding P4/P3/P2/P1 paging levels for
/// this 2 MiB window, but leaves most P1 entries empty. Task 4 uses one of
/// those unused P1 slots for a controlled runtime mapping proof.
pub const RUNTIME_SCRATCH_PAGE_VIRT_ADDR: u64 = 0xffff_ffff_8020_9000;

/// One reserved higher-half slot for future one-page MMIO mappings.
pub const RUNTIME_MMIO_PAGE_VIRT_ADDR: u64 = 0xffff_ffff_8020_a000;

/// Reports failure while walking or mutating the current x86_64 page tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapError {
    UnalignedVirtualAddress,
    UnalignedPhysicalAddress,
    MissingIntermediateTable,
    PageAlreadyMapped,
    PageNotMapped,
    PageTableNotIdentityMapped,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        return Self(0);
    }

    pub const fn is_present(self) -> bool {
        return self.0 & PRESENT != 0;
    }

    pub const fn address(self) -> PhysicalAddress {
        return PhysicalAddress::new(self.0 & 0x000f_ffff_ffff_f000);
    }

    pub fn set_frame(&mut self, frame: PhysicalFrame, flags: u64) {
        self.0 = frame.start_address().as_u64() | flags;
    }

    pub fn set_address(&mut self, addr: PhysicalAddress, flags: u64) {
        self.0 = addr.as_u64() | flags;
    }
}

#[repr(C, align(4096))]
struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    fn entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        return &mut self.entries[index];
    }
}

fn p4_index(addr: u64) -> usize {
    return ((addr >> 39) & 0x1ff) as usize;
}

fn p3_index(addr: u64) -> usize {
    return ((addr >> 30) & 0x1ff) as usize;
}

fn p2_index(addr: u64) -> usize {
    return ((addr >> 21) & 0x1ff) as usize;
}

fn p1_index(addr: u64) -> usize {
    return ((addr >> 12) & 0x1ff) as usize;
}

/// Returns the active bootstrap P4 table through its still-live identity alias.
fn active_p4_table() -> Result<&'static mut PageTable, MapError> {
    return page_table_from_identity_mapped_phys(reserved::p4_table_phys());
}

/// Reinterprets one bootstrap page-table physical page through the existing
/// low identity mapping established during early bring-up.
fn page_table_from_identity_mapped_phys(
    phys_addr: PhysicalAddress,
) -> Result<&'static mut PageTable, MapError> {
    if !phys_addr.is_aligned() {
        return Err(MapError::UnalignedPhysicalAddress);
    }

    if phys_addr.as_u64() >= IDENTITY_MAP_LIMIT {
        return Err(MapError::PageTableNotIdentityMapped);
    }

    let table = phys_addr.as_u64() as *mut PageTable;

    // Sound because bootstrap page tables still reside in the first 2 MiB
    // identity-mapped window, and this helper is only used to reinterpret
    // those 4 KiB-aligned paging structures by address.
    return Ok(unsafe { &mut *table });
}

/// Follows one present paging entry to the next lower-level page table.
fn next_table(entry: PageTableEntry) -> Result<&'static mut PageTable, MapError> {
    if !entry.is_present() {
        return Err(MapError::MissingIntermediateTable);
    }

    return page_table_from_identity_mapped_phys(entry.address());
}

fn p1_table_for_virtual_address(virt_addr: u64) -> Result<&'static mut PageTable, MapError> {
    let p4 = active_p4_table()?;
    let p3 = next_table(*p4.entry_mut(p4_index(virt_addr)))?;
    let p2 = next_table(*p3.entry_mut(p3_index(virt_addr)))?;
    return next_table(*p2.entry_mut(p2_index(virt_addr)));
}

fn flush_page(virt_addr: u64) {
    // Sound because invalidating one page-table translation for a known virtual
    // address is required after changing its current P1 entry.
    unsafe { core::arch::asm!("invlpg [{}]", in(reg) virt_addr, options(nostack, preserves_flags)) }
}

/// Maps one 4 KiB kernel page at `virt_addr` to `frame` as present+writable.
pub fn map_kernel_page(virt_addr: u64, frame: PhysicalFrame) -> Result<(), MapError> {
    if PhysicalAddress::new(virt_addr).is_aligned() == false {
        return Err(MapError::UnalignedVirtualAddress);
    }

    let p1 = p1_table_for_virtual_address(virt_addr)?;
    let entry = p1.entry_mut(p1_index(virt_addr));

    if entry.is_present() {
        return Err(MapError::PageAlreadyMapped);
    }

    entry.set_frame(frame, PRESENT | WRITABLE);
    flush_page(virt_addr);
    return Ok(());
}

/// Maps one 4 KiB MMIO page at `virt_addr` to the aligned physical `phys_addr`.
pub fn map_mmio_page(virt_addr: u64, phys_addr: PhysicalAddress) -> Result<(), MapError> {
    if PhysicalAddress::new(virt_addr).is_aligned() == false {
        return Err(MapError::UnalignedVirtualAddress);
    }

    if phys_addr.is_aligned() == false {
        return Err(MapError::UnalignedPhysicalAddress);
    }

    let p1 = p1_table_for_virtual_address(virt_addr)?;
    let entry = p1.entry_mut(p1_index(virt_addr));

    if entry.is_present() {
        return Err(MapError::PageAlreadyMapped);
    }

    entry.set_address(
        phys_addr,
        PRESENT | WRITABLE | WRITE_THROUGH | CACHE_DISABLE,
    );
    flush_page(virt_addr);
    return Ok(());
}

/// Removes one present 4 KiB kernel or MMIO mapping from the current P1 table.
pub fn unmap_kernel_page(virt_addr: u64) -> Result<(), MapError> {
    if PhysicalAddress::new(virt_addr).is_aligned() == false {
        return Err(MapError::UnalignedVirtualAddress);
    }

    let p1 = p1_table_for_virtual_address(virt_addr)?;
    let entry = p1.entry_mut(p1_index(virt_addr));

    if !entry.is_present() {
        return Err(MapError::PageNotMapped);
    }

    *entry = PageTableEntry::empty();
    flush_page(virt_addr);
    return Ok(());
}
