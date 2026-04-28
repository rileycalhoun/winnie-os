const ISA_DEBUG_EXIT_PORT: u16 = 0xF4;

/// Exit codes written to QEMU's `isa-debug-exit` device during test runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

/// Requests that QEMU terminate the current guest with a deterministic test code.
///
/// This only works when QEMU is launched with the `isa-debug-exit` device bound
/// to the matching I/O port. If the device is absent, the guest falls back to
/// the ordinary terminal halt path after the port write.
pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
    // Sound because the `isa-debug-exit` device interprets one 32-bit port
    // write on `ISA_DEBUG_EXIT_PORT` as a guest-directed exit request in the
    // QEMU test environment.
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") ISA_DEBUG_EXIT_PORT,
            in("eax") exit_code as u32,
            options(nomem, nostack, preserves_flags),
        );
    }

    crate::hlt_loop()
}
