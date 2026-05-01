use core::{
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{arch::x86_64::paging, memory::PhysicalAddress};

const IA32_APIC_BASE_MSR: u32 = 0x1b;
const IA32_APIC_BASE_ENABLE_BIT: u64 = 1 << 11;
const IA32_APIC_BASE_PHYS_MASK: u64 = 0x000f_ffff_ffff_f000;

const SPURIOUS_INTERRUPT_VECTOR_REGISTER_OFFSET: u64 = 0x0f0;
const END_OF_INTERRUPT_REGISTER_OFFSET: u64 = 0x0b0;
const SPURIOUS_VECTOR: u32 = 0xff;
const SPURIOUS_INTERRUPT_VECTOR_ENABLE_BIT: u32 = 1 << 8;

/// Serial marker emitted once the LAPIC MMIO path is mapped and enabled.
pub const LAPIC_INIT_MARKER: &str = "LAPIC INIT OK";

static LAPIC_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Reports failure while mapping or enabling the local APIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicInitError {
    MappedPageNotAligned,
    MmioMapFailed(paging::MapError),
}

/// Maps and enables the LAPIC through the runtime MMIO mapper.
pub fn initialize() -> Result<(), ApicInitError> {
    let mut apic_base = read_apic_base_msr();

    if apic_base & IA32_APIC_BASE_ENABLE_BIT == 0 {
        apic_base |= IA32_APIC_BASE_ENABLE_BIT;
        write_apic_base_msr(apic_base);
    }

    let lapic_phys_base = PhysicalAddress::new(apic_base & IA32_APIC_BASE_PHYS_MASK);
    if !lapic_phys_base.is_aligned() {
        return Err(ApicInitError::MappedPageNotAligned);
    }

    paging::map_mmio_page(paging::RUNTIME_MMIO_PAGE_VIRT_ADDR, lapic_phys_base)
        .map_err(ApicInitError::MmioMapFailed)?;

    let spurious_value = read_register(SPURIOUS_INTERRUPT_VECTOR_REGISTER_OFFSET);
    write_register(
        SPURIOUS_INTERRUPT_VECTOR_REGISTER_OFFSET,
        spurious_value | SPURIOUS_INTERRUPT_VECTOR_ENABLE_BIT | SPURIOUS_VECTOR,
    );

    LAPIC_INITIALIZED.store(true, Ordering::Relaxed);
    Ok(())
}

/// Reports whether the LAPIC MMIO path has been initialized already.
pub fn is_initialized() -> bool {
    LAPIC_INITIALIZED.load(Ordering::Relaxed)
}

/// Sends end-of-interrupt to the LAPIC after one handled external interrupt.
pub fn end_of_interrupt() {
    write_register(END_OF_INTERRUPT_REGISTER_OFFSET, 0);
}

fn register_ptr(offset: u64) -> *mut u32 {
    return (paging::RUNTIME_MMIO_PAGE_VIRT_ADDR + offset) as *mut u32;
}

fn read_register(offset: u64) -> u32 {
    let register = register_ptr(offset);

    // Sound because `initialize()` maps the LAPIC MMIO page at the fixed
    // runtime MMIO slot before any register access, and each LAPIC register is
    // read through its architecturally defined 32-bit offset.
    return unsafe { ptr::read_volatile(register) };
}

fn write_register(offset: u64, value: u32) {
    let register = register_ptr(offset);

    // Sound because `initialize()` maps the LAPIC MMIO page at the fixed
    // runtime MMIO slot before any register access, and each LAPIC register is
    // written through its architecturally defined 32-bit offset.
    unsafe { ptr::write_volatile(register, value) }
}

fn read_apic_base_msr() -> u64 {
    let low: u32;
    let high: u32;

    // Sound because `rdmsr` reads the architecturally defined IA32_APIC_BASE
    // MSR to discover the LAPIC physical base and global enable bit.
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") IA32_APIC_BASE_MSR,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }

    return ((high as u64) << 32) | (low as u64);
}

fn write_apic_base_msr(value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    // Sound because `wrmsr` updates the architecturally defined IA32_APIC_BASE
    // MSR while preserving its existing physical-base field and BSP state.
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") IA32_APIC_BASE_MSR,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
}
