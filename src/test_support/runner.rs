use crate::{
    println,
    test_support::qemu::{QemuExitCode, exit_qemu},
};

/// Stable serial-visible marker emitted before a test suite begins running.
pub const TEST_SUITE_START_MARKER: &str = "TEST SUITE START";

/// Stable serial-visible marker emitted after one test case succeeds.
pub const TEST_CASE_PASS_MARKER: &str = "PASS";

/// Stable serial-visible marker emitted before the test runner exits with failure.
pub const TEST_CASE_FAIL_MARKER: &str = "FAIL";

/// Stable serial-visible marker emitted before the test runner exits successfully.
pub const TEST_SUITE_OK_MARKER: &str = "TEST SUITE OK";

/// Minimal interface implemented by callable kernel test cases.
pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    /// Runs one test case and reports a passing result if it returns normally.
    fn run(&self) {
        self();
        report_test_case_pass();
    }
}

/// Emits the stable start-of-suite marker.
pub fn report_test_suite_start() {
    println!("{}", TEST_SUITE_START_MARKER);
}

/// Emits the stable per-test passing marker.
pub fn report_test_case_pass() {
    println!("{}", TEST_CASE_PASS_MARKER);
}

/// Emits the stable per-test failure marker and exits QEMU with failure.
pub fn report_test_case_fail() -> ! {
    println!("{}", TEST_CASE_FAIL_MARKER);
    exit_qemu(QemuExitCode::Failed)
}

/// Emits the stable suite-success marker and exits QEMU successfully.
pub fn report_test_suite_success() -> ! {
    println!("{}", TEST_SUITE_OK_MARKER);
    exit_qemu(QemuExitCode::Success)
}
