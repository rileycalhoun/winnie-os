const PIC1_DATA_PORT: u16 = 0x21;
const PIC2_DATA_PORT: u16 = 0xa1;

/// Serial marker emitted once the kernel has explicitly masked the legacy PIC.
pub const PIC_INIT_MARKER: &str = "PIC INIT OK";

/// Masks all legacy PIC IRQ lines so the kernel owns the transition away from
/// the bootstrap-era default controller state explicitly.
pub fn mask_all() {
    // Sound because the PIC data ports are fixed x86 legacy controller ports,
    // and writing `0xff` masks every external IRQ line on each chip.
    unsafe {
        outb(PIC1_DATA_PORT, 0xff);
        outb(PIC2_DATA_PORT, 0xff);
    }
}

unsafe fn outb(port: u16, value: u8) {
    // Sound because the caller chooses a valid legacy PIC I/O port and byte
    // value for this programmed port write.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags),
        );
    }
}
