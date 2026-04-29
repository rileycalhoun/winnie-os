/// Shared QEMU-specific test-environment helpers.
pub use qemu::{QemuExitCode, exit_qemu};

/// Shared test runner traits and serial-visible result markers.
pub use runner::{
    Testable, report_test_case_fail, report_test_case_pass, report_test_suite_start,
    report_test_suite_success,
};

pub use scenarios::{BootScenario, selected_boot_scenario};

/// QEMU-only helpers for deterministic guest-directed process exit during tests.
pub mod qemu;

/// Shared test result reporting helpers used by later kernel test harness code.
pub mod runner;

/// Compile-time boot-scenario selection for the harness and destructive tests.
pub mod scenarios;

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
