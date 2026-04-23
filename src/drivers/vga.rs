use core::fmt;

const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const COLOR: u8 = 0x0F; // white on black

static mut COL: usize = 0;
static mut ROW: usize = 0;

fn write_byte(byte: u8) {
    unsafe {
        match byte {
            b'\n' => newline(),
            byte => {
                if COL >= WIDTH {
                    newline();
                }
                let offset = (ROW * WIDTH + COL) * 2;
                *VGA_BUFFER.add(offset) = byte;
                *VGA_BUFFER.add(offset + 1) = COLOR;
                COL += 1;
            }
        }
    }
}

fn newline() {
    unsafe {
        COL = 0;
        ROW += 1;
        if ROW >= HEIGHT {
            ROW = HEIGHT - 1;
        }
    }
}

pub struct Writer;

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            write_byte(byte);
        }
        Ok(())
    }
}

pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    Writer.write_fmt(args).ok();
}
