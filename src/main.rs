#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(winnie_os::test_support::runner::test_runner)]
#![reexport_test_harness_main = "test_main"]

/// Bridges the architecture bootstrap handoff into either the normal kernel
/// runtime or the bootable test harness, depending on the current build mode.
///
/// This function performs the common early bring-up shared by both paths:
/// initialize serial output, load the IDT, and fall back to VGA-only reporting
/// if serial initialization fails. Test builds then dispatch into the generated
/// `test_main`, while non-test builds continue into [`winnie_os::kernel_main`].
#[unsafe(no_mangle)]
extern "C" fn kernel_main_high(multiboot_magic: u32, multiboot_info_addr: usize) -> ! {
    let serial_ready = winnie_os::drivers::serial::init().is_ok();
    winnie_os::arch::x86_64::idt::init();

    if !serial_ready {
        winnie_os::drivers::vga::write_bytes(b"[serial init failed]\n");
    }

    #[cfg(test)]
    {
        let _ = (multiboot_magic, multiboot_info_addr);
        test_main();
        winnie_os::hlt_loop()
    }

    #[cfg(not(test))]
    {
        winnie_os::kernel_main(multiboot_magic, multiboot_info_addr)
    }
}

/// Handles any kernel panic with an intentionally minimal terminal path.
///
/// This function runs whenever Rust triggers a panic after the kernel has reached
/// the point where the panic handler is available. The current implementation
/// assumes machine state may already be compromised, so it avoids complex recovery,
/// richer panic formatting, or any attempt to continue execution.
///
/// Its concrete behavior is limited to printing `PANIC` and then entering
/// [`hlt_loop`]. It never returns because the kernel treats panic as fatal and
/// chooses a simple, auditable halt path over speculative recovery.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    #[cfg(test)]
    {
        winnie_os::println!("{}", winnie_os::test_support::PANIC_MARKER);
        winnie_os::test_support::report_test_case_fail()
    }

    #[cfg(not(test))]
    winnie_os::panic_halt()
}

#[cfg(test)]
/// Minimal smoke test proving the bootable kernel test harness can execute.
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
