#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unconditional_panic, unconditional_recursion)]

mod arch;
mod drivers;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::vga::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[inline(never)]
fn stack_overflow() {
    let x = 0u64;
    core::hint::black_box(x);
    stack_overflow();
}

#[unsafe(no_mangle)]
extern "C" fn kernel_main_high() -> ! {
    arch::x86_64::idt::init();
    println!("Hello from WinnieOS!");

    stack_overflow();
    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("PANIC: {}", info.message());
    loop {}
}
