pub const MAX_MEMORY_REGIONS: usize = 64;

/// Classifies one boot-time physical memory region.
///
/// These variants intentionally stay close to the Multiboot2 memory-map kinds
/// because Phase 0 only needs a small owned handoff for later memory-manager
/// work, not a broader bootloader abstraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionKind {
    Usable,
    Reserved,
    AcpiReclaimable,
    Nvs,
    BadMemory,
    Unknown(u32),
}

impl MemoryRegionKind {
    /// Converts one Multiboot2 memory-map type code into the owned kernel form.
    pub const fn from_multiboot_type(raw: u32) -> Self {
        match raw {
            1 => Self::Usable,
            2 => Self::Reserved,
            3 => Self::AcpiReclaimable,
            4 => Self::Nvs,
            5 => Self::BadMemory,
            other => Self::Unknown(other),
        }
    }

    /// Returns a stable short string for boot-time memory-map logging.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Usable => "usable",
            Self::Reserved => "reserved",
            Self::AcpiReclaimable => "acpi-reclaimable",
            Self::Nvs => "nvs",
            Self::BadMemory => "bad-memory",
            Self::Unknown(_) => "unknown",
        }
    }
}

/// Describes one owned boot-time physical memory region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    pub base: u64,
    pub length: u64,
    pub kind: MemoryRegionKind,
}

const EMPTY_REGION: MemoryRegion = MemoryRegion {
    base: 0,
    length: 0,
    kind: MemoryRegionKind::Reserved,
};

/// Reports failure while populating the owned `BootInfo` structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootInfoError {
    TooManyMemoryRegions,
}

/// Holds the small owned boot metadata Phase 0 preserves after handoff.
///
/// The structure keeps only the currently needed memory-map data in fixed
/// storage so later kernel code does not depend on raw Multiboot2 pointers.
pub struct BootInfo {
    regions: [MemoryRegion; MAX_MEMORY_REGIONS],
    region_count: usize,
}

impl BootInfo {
    /// Creates an empty boot-info container with fixed backing storage.
    pub const fn new() -> Self {
        Self {
            regions: [EMPTY_REGION; MAX_MEMORY_REGIONS],
            region_count: 0,
        }
    }

    /// Appends one parsed memory region to the owned boot-info list.
    ///
    /// This fails instead of silently truncating when the fixed-capacity region
    /// buffer is exhausted.
    pub fn push_region(&mut self, region: MemoryRegion) -> Result<(), BootInfoError> {
        if self.region_count >= MAX_MEMORY_REGIONS {
            return Err(BootInfoError::TooManyMemoryRegions);
        }

        self.regions[self.region_count] = region;
        self.region_count += 1;
        Ok(())
    }

    /// Returns only the populated prefix of parsed memory regions.
    pub fn regions(&self) -> &[MemoryRegion] {
        return &self.regions[..self.region_count];
    }

    /// Reports how many memory regions are currently populated.
    pub const fn region_count(&self) -> usize {
        return self.region_count;
    }

    /// Reports whether the boot-info structure currently holds any regions.
    #[allow(dead_code)]
    pub const fn is_empty(&self) -> bool {
        return self.region_count == 0;
    }
}
