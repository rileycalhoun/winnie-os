use crate::memory::PhysicalAddress;

unsafe extern "C" {
    static __boot_phys_start: u8;
    static __boot_phys_end: u8;
    static __kernel_phys_start: u8;
    static __kernel_phys_end: u8;

    static kernel_stack_page0: u8;
    static kernel_stack_page1: u8;
    static pf_ist_stack_page: u8;
    static df_ist_stack_page: u8;

    static p4_table: u8;
    static p3_low: u8;
    static p2_low: u8;
    static p1_low: u8;
    static p3_high: u8;
    static p2_high: u8;
    static p1_high_kernel: u8;
    static p1_high_stack: u8;
}

pub fn boot_phys_start() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(__boot_phys_start) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the first byte past the low bootstrap physical span.
pub fn boot_phys_end() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(__boot_phys_end) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical start of the higher-half kernel image.
pub fn kernel_phys_start() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(__kernel_phys_start) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the first byte past the higher-half kernel image in physical memory.
pub fn kernel_phys_end() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(__kernel_phys_end) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical backing page of the first mapped kernel stack page.
pub fn kernel_stack_page0_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(kernel_stack_page0) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical backing page of the second mapped kernel stack page.
pub fn kernel_stack_page1_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(kernel_stack_page1) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical backing page of the page-fault IST stack.
pub fn pf_ist_stack_page_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(pf_ist_stack_page) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical backing page of the double-fault IST stack.
pub fn df_ist_stack_page_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(df_ist_stack_page) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing the top-level bootstrap page table.
pub fn p4_table_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p4_table) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing the low identity-mapped P3 table.
pub fn p3_low_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p3_low) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing the low identity-mapped P2 table.
pub fn p2_low_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p2_low) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing the low identity-mapped P1 table.
pub fn p1_low_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p1_low) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing the higher-half P3 table.
pub fn p3_high_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p3_high) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing the higher-half P2 table.
pub fn p2_high_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p2_high) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing higher-half kernel-image P1 entries.
pub fn p1_high_kernel_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p1_high_kernel) as u64;
    return PhysicalAddress::new(addr);
}

/// Returns the physical page containing higher-half stack-window P1 entries.
pub fn p1_high_stack_phys() -> PhysicalAddress {
    let addr = core::ptr::addr_of!(p1_high_stack) as u64;
    return PhysicalAddress::new(addr);
}
