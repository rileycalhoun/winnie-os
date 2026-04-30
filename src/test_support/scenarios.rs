use core::sync::atomic::{AtomicU8, Ordering};

/// Compile-time-selected boot modes for the bootable kernel test harness and
/// dedicated destructive test kernels.
#[repr(u8)]
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

impl BootScenario {
    const fn from_repr(value: u8) -> Self {
        match value {
            value if value == Self::Normal as u8 => Self::Normal,
            value if value == Self::TestHarness as u8 => Self::TestHarness,
            value if value == Self::Panic as u8 => Self::Panic,
            value if value == Self::InvalidOpcode as u8 => Self::InvalidOpcode,
            value if value == Self::GeneralProtection as u8 => Self::GeneralProtection,
            value if value == Self::PageFault as u8 => Self::PageFault,
            value if value == Self::DoubleFault as u8 => Self::DoubleFault,
            _ => Self::Normal,
        }
    }
}

static ACTIVE_BOOT_SCENARIO: AtomicU8 = AtomicU8::new(BootScenario::Normal as u8);

/// Returns the single boot scenario selected for the current build.
///
/// Destructive test features take precedence so each dedicated fault run
/// produces exactly one intended behavior.
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

    return BootScenario::Normal;
}

/// Publishes the boot scenario chosen by the binary entrypoint.
pub fn set_active_boot_scenario(scenario: BootScenario) {
    ACTIVE_BOOT_SCENARIO.store(scenario as u8, Ordering::Relaxed);
}

/// Returns the boot scenario currently active for panic and fault handlers.
pub fn active_boot_scenario() -> BootScenario {
    BootScenario::from_repr(ACTIVE_BOOT_SCENARIO.load(Ordering::Relaxed))
}
