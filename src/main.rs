#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unconditional_panic, unconditional_recursion)]

mod arch;
mod console;
mod drivers;

/// Enters the kernel's terminal halt path by repeatedly executing `hlt`.
///
/// This function is used once the kernel has no further work to do or has reached
/// an unrecoverable terminal state, which is the current end of control flow both
/// after normal startup in [`kernel_main_high`] and after a panic in [`panic()`].
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

/// Runs as the higher-half Rust entrypoint after the architecture bootstrap code
/// has finished entering long mode and transferring control into the kernel.
///
/// At this point the early boot path has already established the current machine
/// state needed to execute Rust in the higher half, including the active kernel
/// stack and the basic descriptor and paging setup performed before this handoff.
/// This function first initializes the early serial debug path, then loads the
/// IDT so exception handling is in place before emitting the current startup
/// output. If serial initialization fails, the function reports that condition
/// on VGA and continues with VGA-only console mirroring.
///
/// After printing `Hello from WinnieOS!`, it hands control to [`hlt_loop`], which
/// is the kernel's current terminal path. It never returns because there is no
/// scheduler, idle task, or later boot stage to return to in the current system.
#[unsafe(no_mangle)]
extern "C" fn kernel_main_high() -> ! {
    let serial_ready = drivers::serial::init().is_ok();
    arch::x86_64::idt::init();

    if !serial_ready {
        crate::drivers::vga::write_bytes(b"[serial init failed]\n");
    }

    println!("Hello from WinnieOS!");
    hlt_loop()
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
    println!("PANIC");
    hlt_loop()
}
