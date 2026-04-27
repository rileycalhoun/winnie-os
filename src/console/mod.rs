use core::fmt;

/// Formats text for the kernel console without appending a trailing newline.
///
/// This macro packages its token input with [`format_args!`] and forwards the
/// resulting formatting arguments into the console print bridge.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::console::_print(format_args!($($arg)*)));
}

/// Formats text for the kernel console and appends a trailing newline.
///
/// This is the newline-adding companion to [`print!`], and it forwards its
/// formatted output through the same console path.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

struct ConsoleWriter;

impl fmt::Write for ConsoleWriter {
    /// Mirrors formatted string fragments to each currently active early sink.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::drivers::vga::write_bytes(s.as_bytes());
        if crate::drivers::serial::is_enabled() {
            crate::drivers::serial::write_bytes(s.as_bytes());
        }

        Ok(())
    }
}

/// Bridges console formatting macros to the current early console sinks.
///
/// This function receives a prebuilt [`fmt::Arguments`] value produced by
/// `format_args!` in the exported console macros. It does not own cursor state,
/// device state, or any buffering logic itself; instead, it formats once and
/// mirrors the resulting output to both the VGA and serial backends used during
/// early kernel bring-up.
pub fn _print(args: fmt::Arguments) {
    use fmt::Write;
    if ConsoleWriter.write_fmt(args).is_err() {
        crate::drivers::vga::write_bytes(b"[console fmt error]");

        if crate::drivers::serial::is_enabled() {
            crate::drivers::serial::write_bytes(b"[console fmt error]");
        }
    }
}
