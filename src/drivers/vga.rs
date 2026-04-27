const VGA_BUFFER: *mut u8 = 0xB8000 as *mut u8;
const WIDTH: usize = 80;
const HEIGHT: usize = 25;
const COLOR: u8 = 0x0F; // white on black

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
pub fn write_bytes(bytes: &[u8]) {
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
