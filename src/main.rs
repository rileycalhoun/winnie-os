#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unconditional_panic)]

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

#[unsafe(no_mangle)]
extern "C" fn kernel_main() -> ! {
    arch::x86_64::idt::init();
    println!("Hello from WinnieOS!");

    unsafe { core::arch::asm!("ud2") };
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!("PANIC");
    loop {}
}
