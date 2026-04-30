use crate::{
    boot_info::{BootInfo, MAX_MEMORY_REGIONS, MemoryRegionKind},
    memory::{PhysicalAddress, PhysicalFrame},
};

/// Reports failure while normalizing usable boot-time regions into frame ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatorInitError {
    RegionOverflow,
    BadFrameRegion,
    TooManyUsableRegions,
}

/// One normalized usable physical-frame interval stored by the allocator.
#[derive(Clone, Copy)]
struct UsableFrameRegion {
    start: PhysicalFrame,
    end_exclusive: PhysicalFrame,
}

/// Sentinel value used to initialize the unused tail of the fixed region array.
const EMPTY_USABLE_FRAME_REGION: UsableFrameRegion = UsableFrameRegion {
    start: PhysicalFrame::from_start_address(PhysicalAddress::new(0)).unwrap(),
    end_exclusive: PhysicalFrame::from_start_address(PhysicalAddress::new(0)).unwrap(),
};

/// Monotonically allocates 4 KiB physical frames from owned usable boot regions.
pub struct MonotonicFrameAllocator {
    regions: [UsableFrameRegion; MAX_MEMORY_REGIONS],
    region_count: usize,
    current_region: usize,
    next_frame: Option<PhysicalFrame>,
}

impl MonotonicFrameAllocator {
    /// Creates an empty allocator with no usable frame regions loaded yet.
    pub const fn empty() -> Self {
        Self {
            regions: [EMPTY_USABLE_FRAME_REGION; MAX_MEMORY_REGIONS],
            region_count: 0,
            current_region: 0,
            next_frame: None,
        }
    }

    /// Builds allocator-owned usable frame ranges from the parsed boot memory map.
    ///
    /// The constructor keeps only `Usable` regions, normalizes them to 4 KiB
    /// frame boundaries, discards post-alignment empty ranges, and initializes
    /// the allocator cursor to the first allocatable frame if one exists.
    pub fn initialize_from_boot_info(
        &mut self,
        boot_info: &BootInfo,
    ) -> Result<(), AllocatorInitError> {
        self.regions = [EMPTY_USABLE_FRAME_REGION; MAX_MEMORY_REGIONS];
        self.region_count = 0;
        self.current_region = 0;
        self.next_frame = None;

        for region in boot_info
            .regions()
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
        {
            let raw_start = PhysicalAddress::new(region.base);
            let raw_end = PhysicalAddress::new(
                region
                    .base
                    .checked_add(region.length)
                    .ok_or(AllocatorInitError::RegionOverflow)?,
            );

            let aligned_start = raw_start
                .checked_align_up()
                .ok_or(AllocatorInitError::RegionOverflow)?;
            let aligned_end = raw_end.align_down();

            if aligned_start.as_u64() >= aligned_end.as_u64() {
                continue;
            }

            let start = PhysicalFrame::from_start_address(aligned_start)
                .ok_or(AllocatorInitError::BadFrameRegion)?;
            let end_exclusive = PhysicalFrame::from_start_address(aligned_end)
                .ok_or(AllocatorInitError::BadFrameRegion)?;

            if self.region_count >= MAX_MEMORY_REGIONS {
                return Err(AllocatorInitError::TooManyUsableRegions);
            }

            self.regions[self.region_count] = UsableFrameRegion {
                start,
                end_exclusive,
            };
            self.region_count += 1;
        }

        if self.region_count != 0 {
            self.next_frame = Some(self.regions[0].start);
        }

        return Ok(());
    }

    /// Builds a fresh monotonic allocator from the parsed boot memory map.
    pub fn new(boot_info: &BootInfo) -> Result<Self, AllocatorInitError> {
        let mut allocator = Self::empty();
        allocator.initialize_from_boot_info(boot_info)?;
        return Ok(allocator);
    }

    /// Returns the next allocatable 4 KiB frame and advances the internal cursor.
    pub fn allocate_frame(&mut self) -> Option<PhysicalFrame> {
        if self.next_frame.is_none() {
            return None;
        }

        if self.current_region >= self.region_count {
            self.next_frame = None;
            return None;
        }

        let region = self.regions[self.current_region];
        let allocated = self.next_frame.unwrap();
        let next = allocated.checked_next();
        match next {
            Some(candidate) => {
                if candidate.start_address() < region.end_exclusive.start_address() {
                    self.next_frame = Some(candidate);
                    return Some(allocated);
                } else {
                    self.current_region += 1;
                    if self.current_region < self.region_count {
                        self.next_frame = Some(self.regions[self.current_region].start);
                    } else {
                        self.next_frame = None;
                    }

                    return Some(allocated);
                }
            }
            None => {
                self.next_frame = None;
                return Some(allocated);
            }
        }
    }
}
