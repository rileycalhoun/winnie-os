#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unconditional_panic, unconditional_recursion)]

mod arch;
mod console;
mod drivers;

#[unsafe(no_mangle)]
extern "C" fn kernel_main_high() -> ! {
    arch::x86_64::idt::init();
    println!("Hello from WinnieOS!");

    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info.message());
    loop {}
}
