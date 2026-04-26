use core::fmt;

/// Formats text for the kernel console without appending a trailing newline.
///
/// This macro packages its token input with [`format_args!`] and forwards the
/// resulting formatting arguments into the console print bridge.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::drivers::vga::_print(format_args!($($arg)*)));
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

/// Bridges console formatting macros to the VGA text-mode writer.
///
/// This function receives a prebuilt [`fmt::Arguments`] value produced by
/// `format_args!` in the exported console macros. It does not own cursor state,
/// screen memory, or any buffering logic itself; instead, it immediately
/// delegates the formatted output to the VGA backend in `crate::drivers::vga`.
pub fn _print(args: fmt::Arguments) {
    crate::drivers::vga::_print(args);
}
