use core::fmt;

const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const COLOR: u8 = 0x0F; // white on black
const WRITE_ERROR_MESSAGE: &[u8] = b"[vga fmt error]";

static mut COL: usize = 0;
static mut ROW: usize = 0;

/// Writes one byte into the early VGA text-mode console backend.
///
/// This is the byte-level output primitive used during single-threaded kernel
/// bring-up before any richer console or terminal subsystem exists. A newline
/// byte is handled specially by delegating to [`newline`]. Any other byte is
/// written at the current `(ROW, COL)` cursor position, and the column is then
/// advanced by one cell.
///
/// The implementation assumes the VGA text buffer is mapped at `0xB8000` and
/// accessible through the current paging setup. It also assumes this backend is
/// the only code mutating the global cursor state, so there is no locking or
/// synchronization. When the current column reaches the 80-column width, it
/// first moves to the next line before writing. The current implementation does
/// not scroll: once output reaches the bottom row, later writes remain clamped
/// to that row.
fn write_byte(byte: u8) {
    match byte {
        b'\n' => newline(),
        byte => {
            // Sound because VGA output is single-threaded during early kernel execution.
            if unsafe { COL } >= WIDTH {
                newline();
            }
            // Sound because `ROW` and `COL` are clamped to the visible 80x25 VGA text buffer.
            let offset = unsafe { (ROW * WIDTH + COL) * 2 };
            // Sound because `offset` and `offset + 1` stay within the memory-mapped VGA text buffer.
            unsafe { *VGA_BUFFER.add(offset) = byte };
            // Sound because `offset` and `offset + 1` stay within the memory-mapped VGA text buffer.
            unsafe { *VGA_BUFFER.add(offset + 1) = COLOR };
            // Sound because VGA output is single-threaded during early kernel execution.
            unsafe { COL += 1 };
        }
    }
}

/// Writes a byte slice through the VGA backend one byte at a time.
///
/// This keeps all cursor movement and newline behavior centralized in
/// [`write_byte`], which is the only place that touches the text buffer
/// directly.
fn write_bytes(bytes: &[u8]) {
    for &byte in bytes {
        write_byte(byte);
    }
}

/// Moves the VGA cursor to the start of the next row.
///
/// The current row is advanced by one and the column is reset to zero. If the
/// cursor is already on the last visible VGA row, the implementation clamps it
/// to that bottom row instead of scrolling or clearing the screen. This matches
/// the current early bring-up limitation: output is single-threaded and visible
/// only within the fixed 80x25 text buffer.
fn newline() {
    // Sound because VGA cursor state is only mutated by the single-threaded early kernel printer.
    unsafe { COL = 0 };
    // Sound because VGA cursor state is only mutated by the single-threaded early kernel printer.
    unsafe { ROW += 1 };
    // Sound because VGA cursor state is only mutated by the single-threaded early kernel printer.
    if unsafe { ROW } >= HEIGHT {
        // Sound because clamping `ROW` preserves the invariant that it stays within the visible text buffer.
        unsafe { ROW = HEIGHT - 1 };
    }
}

/// Converts a formatting result into a fixed fallback byte string when needed.
///
/// The VGA backend keeps its failure path independent from `fmt` machinery so a
/// formatting error can still produce a small, direct message on screen.
fn fallback_bytes_for_write_result(result: fmt::Result) -> Option<&'static [u8]> {
    match result {
        Ok(()) => None,
        Err(_) => Some(WRITE_ERROR_MESSAGE),
    }
}

/// Emits a fixed fallback message if formatted VGA output fails.
///
/// This keeps the error path simple and visible during early kernel bring-up,
/// where complex recovery is not desirable.
fn handle_write_result(result: fmt::Result) {
    // Keep the failure path out of `fmt` so console errors remain visible and simple.
    if let Some(message) = fallback_bytes_for_write_result(result) {
        write_bytes(message);
    }
}

/// Stateless adapter that lowers formatted kernel output into VGA byte writes.
///
/// `Writer` does not own screen memory, cursor state, or synchronization. It is
/// a thin bridge used by the early text-mode VGA backend while the kernel is
/// still single-threaded and relies on the global `ROW` and `COL` cursor state.
pub struct Writer;

impl fmt::Write for Writer {
    /// Lowers a formatted string slice into byte-wise VGA writes.
    ///
    /// `core::fmt` calls this method with already formatted `&str` fragments.
    /// The implementation converts each fragment to bytes and feeds them through
    /// [`write_bytes`], which in turn preserves the current newline handling and
    /// cursor advancement in [`write_byte`]. This path assumes the same early
    /// single-threaded environment as the rest of the VGA backend.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_bytes(s.as_bytes());
        Ok(())
    }
}

/// Backend entry for higher-level console printing into the VGA text buffer.
///
/// This function receives the prebuilt [`fmt::Arguments`] produced by the
/// console `print!` and `println!` macros, then delegates formatting to
/// [`Writer`]. It is the current backend used during early kernel bring-up, so
/// it relies on the VGA text buffer mapping, global cursor state, and the
/// assumption that output remains single-threaded. It does not implement
/// scrolling or any broader terminal state beyond the fixed 80x25 VGA buffer.
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    handle_write_result(Writer.write_fmt(args));
}
