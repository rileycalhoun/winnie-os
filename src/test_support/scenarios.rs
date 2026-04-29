/// Compile-time-selected boot modes for the bootable kernel test harness and
/// dedicated destructive test kernels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootScenario {
    Normal,
    TestHarness,
    Panic,
    InvalidOpcode,
    GeneralProtection,
    PageFault,
    DoubleFault,
}

/// Returns the single boot scenario selected for the current build.
///
/// Destructive test features take precedence over the ordinary bootable test
/// harness so each dedicated fault run produces exactly one intended behavior.
pub const fn selected_boot_scenario() -> BootScenario {
    #[cfg(feature = "test-panic")]
    {
        return BootScenario::Panic;
    }
    #[cfg(feature = "test-invalid-opcode")]
    {
        return BootScenario::InvalidOpcode;
    }
    #[cfg(feature = "test-general-protection")]
    {
        return BootScenario::GeneralProtection;
    }
    #[cfg(feature = "test-page-fault")]
    {
        return BootScenario::PageFault;
    }
    #[cfg(feature = "test-double-fault")]
    {
        return BootScenario::DoubleFault;
    }

    #[cfg(test)]
    {
        return BootScenario::TestHarness;
    }

    BootScenario::Normal
}
