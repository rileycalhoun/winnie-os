/// Stable serial-visible marker for a kernel panic path.
pub const PANIC_MARKER: &str = "PANIC";

/// Stable serial-visible marker for the divide-error handler.
pub const DIVIDE_ERROR_MARKER: &str = "DIVIDE ERROR";

/// Stable serial-visible marker for the invalid-opcode handler.
pub const INVALID_OPCODE_MARKER: &str = "INVALID OPCODE";

/// Stable serial-visible marker for the double-fault handler.
pub const DOUBLE_FAULT_MARKER: &str = "DOUBLE FAULT";

/// Stable serial-visible marker for the general-protection handler.
pub const GENERAL_PROTECTION_MARKER: &str = "GENERAL PROTECTION FAULT";

/// Stable serial-visible marker for the page-fault handler.
pub const PAGE_FAULT_MARKER: &str = "PAGE FAULT";
