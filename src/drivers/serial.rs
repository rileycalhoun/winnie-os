use core::sync::atomic::{AtomicBool, Ordering};

const COM1: u16 = 0x3F8;

const SERIAL_INIT_LOOPBACK_ERROR: &[u8] = b"Loopback failed";

const LINE_STATUS_PORT: u16 = COM1 + 5;
const TRANSMIT_READY: u8 = 1 << 5;

static SERIAL_ENABLED: AtomicBool = AtomicBool::new(false);

/// Reports whether the COM1 backend completed its early initialization path.
///
/// The current kernel uses this both as a console sink-selection hint and as a
/// defensive guard inside the serial backend so failed initialization degrades
/// to VGA-only output instead of blocking on UART status polling.
pub fn is_enabled() -> bool {
    SERIAL_ENABLED.load(Ordering::Acquire)
}

/// Initializes the COM1 UART for early kernel debug output.
///
/// This programs a simple 38400-baud 8N1 configuration, runs a loopback
/// self-test, and only enables the serial sink if that self-test succeeds.
/// Callers can use the returned error to fall back to VGA-only output while
/// leaving the serial write path safely disabled.
pub fn init() -> core::result::Result<(), &'static [u8]> {
    SERIAL_ENABLED.store(false, Ordering::Release);

    unsafe { outb(COM1 + 1, 0x00) }; // Disable all interrupts
    unsafe { outb(COM1 + 3, 0x80) }; // Enable DLAB (set baud rate divisor)
    unsafe { outb(COM1 + 0, 0x03) }; // Set divisor to 3 (lo byte) 38400 baud
    unsafe { outb(COM1 + 1, 0x00) }; //                  (hi byte)
    unsafe { outb(COM1 + 3, 0x03) }; // 8 bits, no parity, one stop bit
    unsafe { outb(COM1 + 2, 0xC7) }; // Enable FIFO, clear them, with 14-byte threshold
    unsafe { outb(COM1 + 4, 0x0B) }; // IRQs enabled, RTS/DSR set
    unsafe { outb(COM1 + 4, 0x1E) }; // Set in loopback mode, test the serial chip
    unsafe { outb(COM1 + 0, 0xAE) }; // Test serial chip (send byte 0xAE and check if serial returns same byte)

    if unsafe { inb(COM1 + 0) } != 0xAE {
        return Err(SERIAL_INIT_LOOPBACK_ERROR);
    }

    // If serial is not faulty set it in normal operation mode
    // (not-loopback with IRQs enabled and OUT#1 and OUT#2 bits enabled)
    unsafe { outb(COM1 + 4, 0x0F) };
    SERIAL_ENABLED.store(true, Ordering::Release);
    Ok(())
}

/// Checks whether the UART transmit holding register can accept a new byte.
fn is_transmit_ready() -> bool {
    unsafe { inb(LINE_STATUS_PORT) & TRANSMIT_READY != 0 }
}

/// Writes one byte to COM1 when the serial sink is enabled.
///
/// If serial initialization failed earlier, this becomes a no-op so console
/// writes can continue through VGA without hanging on UART status polling.
pub fn write_byte(byte: u8) {
    if !is_enabled() {
        return;
    }

    while !is_transmit_ready() {}
    unsafe { outb(COM1, byte) }
}

/// Writes a byte slice through the COM1 backend.
pub fn write_bytes(bytes: &[u8]) {
    for &byte in bytes {
        write_byte(byte);
    }
}

unsafe fn outb(port: u16, value: u8) {
    // Sound because the caller chooses a valid UART I/O port and byte value for this programmed port write.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value
        )
    }
}

unsafe fn inb(port: u16) -> u8 {
    let value: u8;

    // Sound because the caller chooses a valid UART I/O port for this programmed port read.
    unsafe {
        core::arch::asm!(
            "in al, dx",
            out("al") value,
            in("dx") port
        );
    }

    value
}
