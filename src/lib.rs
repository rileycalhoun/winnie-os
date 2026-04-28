#![no_std]
#![feature(abi_x86_interrupt)]

use core::cell::UnsafeCell;

use crate::boot_info::BootInfo;
use crate::test_support::PANIC_MARKER;

mod boot_info;

pub mod arch;
pub mod console;
pub mod drivers;
pub mod test_support;

/// Wraps the single early-boot `BootInfo` instance in interior mutability.
///
/// Phase 0 parses boot metadata exactly once during single-threaded bring-up,
/// but storing the owned structure off-stack avoids overflowing the small early
/// kernel stack during parser calls.
struct BootInfoStorage(UnsafeCell<BootInfo>);

// Sound because early kernel bring-up is still single-threaded, so no concurrent
// access exists while this storage is initialized and then read.
unsafe impl Sync for BootInfoStorage {}
static BOOT_INFO: BootInfoStorage = BootInfoStorage(UnsafeCell::new(BootInfo::new()));

/// Enters the kernel's terminal halt path by repeatedly executing `hlt`.
///
/// This function is used once the kernel has no further work to do or has reached
/// an unrecoverable terminal state, which is the current end of control flow both
/// after normal startup in [`kernel_main`] and after a fatal panic through
/// [`panic_halt`].
///
/// It assumes the CPU is already in the higher-half kernel runtime established by
/// earlier bootstrap code and that interrupts, if enabled, may wake the processor
/// between halt instructions. It never returns because the kernel intentionally
/// stays in this loop forever instead of attempting to resume execution after its
/// current terminal path has been reached.
pub fn hlt_loop() -> ! {
    loop {
        // Sound because halting the CPU does not access memory and is the intended terminal kernel path.
        unsafe { core::arch::asm!("hlt") }
    }
}

/// Emits the currently parsed boot-time memory map in a stable debug format.
fn log_boot_info(boot_info: &BootInfo) {
    println!("BOOT INFO: {} regions", boot_info.region_count());
    for region in boot_info.regions() {
        println!(
            "MMAP base={:#018x} len={:#018x} kind={}",
            region.base,
            region.length,
            region.kind.as_str()
        )
    }
}

/// Runs as the higher-half Rust entrypoint after the architecture bootstrap code
/// has finished entering long mode and transferring control into the kernel.
///
/// At this point the early boot path has already established the current machine
/// state needed to execute Rust in the higher half, including the active kernel
/// stack and the basic descriptor and paging setup performed before this handoff.
/// The current binary entrypoint is responsible for initializing serial output
/// and loading the IDT before calling this function. `kernel_main` then parses
/// the bootloader memory map into owned kernel storage, logs that parsed view,
/// and emits the current startup output.
///
/// After logging boot information and printing `Hello from WinnieOS!`, it hands
/// control to [`hlt_loop`], which is the kernel's current terminal path. It
/// never returns because there is no scheduler, idle task, or later boot stage
/// to return to in the current system.
pub fn kernel_main(multiboot_magic: u32, multiboot_info_addr: usize) -> ! {
    // Sound because early kernel bring-up is single-threaded and this storage
    // is initialized exactly once before any later shared access exists.
    let boot_info = unsafe { &mut *BOOT_INFO.0.get() };
    match arch::x86_64::boot_info::parse_multiboot2(multiboot_magic, multiboot_info_addr, boot_info)
    {
        Ok(()) => {}
        Err(error) => {
            println!("BOOT INFO PARSE FAILED: {:?}", error);
            hlt_loop()
        }
    };

    log_boot_info(boot_info);
    println!("Hello from WinnieOS!");
    hlt_loop()
}

/// Handles a fatal kernel panic with the current minimal terminal path.
///
/// This helper keeps the actual panic logic shared between the normal boot
/// binary and future test binaries without placing the `#[panic_handler]`
/// lang item inside the library crate itself.
pub fn panic_halt() -> ! {
    println!("{}", PANIC_MARKER);
    hlt_loop()
}
