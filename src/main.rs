#![no_std]
#![no_main]

mod arch;

#[unsafe(no_mangle)]
extern "C" fn kernel_main() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // TODO: Display panic message here in display buffer
    loop {}
}
