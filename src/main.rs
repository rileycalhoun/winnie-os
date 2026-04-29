#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(winnie_os::test_support::runner::test_runner)]
#![reexport_test_harness_main = "test_main"]

use winnie_os::{
    hlt_loop,
    test_support::{BootScenario, PANIC_MARKER, QemuExitCode},
};

/// Bridges the architecture bootstrap handoff into the normal runtime, the
/// bootable test harness, or one dedicated destructive test scenario.
///
/// This function performs the common early bring-up shared by both paths:
/// initialize serial output, load the IDT, and fall back to VGA-only reporting
/// if serial initialization fails. After that, it dispatches according to the
/// compile-time-selected boot scenario: normal boot continues into
/// [`winnie_os::kernel_main`], test builds may enter the generated `test_main`,
/// and destructive test builds deliberately trigger the selected panic or fault
/// path.
#[unsafe(no_mangle)]
extern "C" fn kernel_main_high(multiboot_magic: u32, multiboot_info_addr: usize) -> ! {
    let serial_ready = winnie_os::drivers::serial::init().is_ok();
    winnie_os::arch::x86_64::idt::init();

    if !serial_ready {
        winnie_os::drivers::vga::write_bytes(b"[serial init failed]\n");
    }

    #[cfg(test)]
    let scenario = BootScenario::TestHarness;

    #[cfg(not(test))]
    let scenario = winnie_os::test_support::selected_boot_scenario();

    match scenario {
        BootScenario::Normal => winnie_os::kernel_main(multiboot_magic, multiboot_info_addr),
        BootScenario::TestHarness => {
            #[cfg(test)]
            {
                let _ = (multiboot_magic, multiboot_info_addr);
                test_main();
                winnie_os::hlt_loop()
            }

            #[cfg(not(test))]
            {
                winnie_os::println!("TEST HARNESS REQUIRES cfg(test)");
                winnie_os::hlt_loop()
            }
        }
        BootScenario::Panic => {
            panic!("expected panic test");
        }
        BootScenario::InvalidOpcode => {
            unsafe {
                core::arch::asm!("ud2");
            }

            hlt_loop()
        }
        BootScenario::GeneralProtection => {
            let ptr = 0x0000_8000_0000_0000 as *mut u64;

            unsafe {
                core::ptr::write_volatile(ptr, 0);
            }

            hlt_loop()
        }
        BootScenario::PageFault => {
            let ptr = 0x0000_4000_0000_0000 as *mut u64;

            unsafe {
                core::ptr::write_volatile(ptr, 0);
            }

            hlt_loop()
        }
        BootScenario::DoubleFault => {
            winnie_os::arch::x86_64::idt::clear_page_fault_handler_for_double_fault_test();
            let ptr = 0x0000_4000_0000_0000 as *mut u64;

            unsafe {
                core::ptr::write_volatile(ptr, 0);
            }

            hlt_loop()
        }
    }
}

/// Handles any kernel panic with either the normal fatal path or the dedicated
/// panic-test success path.
///
/// This function runs whenever Rust triggers a panic after the kernel has reached
/// the point where the panic handler is available. The current implementation
/// assumes machine state may already be compromised, so it avoids complex recovery,
/// richer panic formatting, or any attempt to continue execution.
///
/// Under the dedicated panic-test scenario, the handler treats reaching this
/// path as success and exits QEMU with the passing test code after printing the
/// stable panic marker. In the bootable test harness, panic still reports
/// failure. All other builds use the normal fatal kernel halt path.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    match winnie_os::test_support::selected_boot_scenario() {
        BootScenario::Panic => {
            winnie_os::println!("{}", PANIC_MARKER);
            winnie_os::test_support::exit_qemu(QemuExitCode::Success)
        }
        BootScenario::TestHarness => {
            winnie_os::println!("{}", PANIC_MARKER);
            winnie_os::test_support::report_test_case_fail()
        }
        _ => winnie_os::panic_halt(),
    }
}

#[cfg(test)]
/// Minimal smoke test proving the bootable kernel test harness can execute.
#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
