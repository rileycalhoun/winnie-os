use crate::{
    arch,
    boot_info::{BootInfo, MAX_MEMORY_REGIONS, MemoryRegionKind},
    memory::{FRAME_SIZE, PhysicalAddress, PhysicalFrame},
};

/// Reports failure while normalizing usable boot-time regions into frame ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatorInitError {
    RegionOverflow,
    BadFrameRegion,
    TooManyAllocatableRegions,
}

/// Reports failure while returning one runtime-owned frame to the allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreeError {
    ReservedFrame,
    UnmanagedFrame,
    FrameAlreadyFreed,
    FreedFrameOverflow,
}

/// One normalized usable physical-frame interval stored by the allocator.
#[derive(Clone, Copy)]
struct UsableFrameRegion {
    start: PhysicalFrame,
    end_exclusive: PhysicalFrame,
}

const RESERVED_RANGE_COUNT: usize = 2;
const MAX_REGION_FRAGMENTS: usize = RESERVED_RANGE_COUNT + 1;
const MAX_ALLOCATABLE_FRAME_REGIONS: usize = MAX_MEMORY_REGIONS * MAX_REGION_FRAGMENTS;

/// Sentinel value used to initialize the unused tail of the fixed region array.
const EMPTY_USABLE_FRAME_REGION: UsableFrameRegion = UsableFrameRegion {
    start: PhysicalFrame::from_start_address(PhysicalAddress::new(0)).unwrap(),
    end_exclusive: PhysicalFrame::from_start_address(PhysicalAddress::new(0)).unwrap(),
};

/// Allocates 4 KiB physical frames from owned usable boot regions with LIFO reuse.
pub struct MonotonicFrameAllocator {
    regions: [UsableFrameRegion; MAX_ALLOCATABLE_FRAME_REGIONS],
    region_count: usize,
    current_region: usize,
    next_frame: Option<PhysicalFrame>,
    freed_frames: [Option<PhysicalFrame>; MAX_ALLOCATABLE_FRAME_REGIONS],
    freed_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PhysicalRange {
    start: PhysicalAddress,
    end_exclusive: PhysicalAddress,
}

/// Builds one physical half-open range `[start, end_exclusive)`.
fn physical_range(start: PhysicalAddress, end_exclusive: PhysicalAddress) -> PhysicalRange {
    return PhysicalRange {
        start,
        end_exclusive,
    };
}

/// Collects the current kernel-owned physical spans that the allocator must skip.
///
/// Task 3 starts by reserving:
/// - the full low bootstrap physical span
/// - the higher-half kernel image physical span
fn collect_reserved_ranges() -> [PhysicalRange; RESERVED_RANGE_COUNT] {
    return [
        physical_range(
            arch::x86_64::reserved::boot_phys_start(),
            arch::x86_64::reserved::boot_phys_end(),
        ),
        physical_range(
            arch::x86_64::reserved::kernel_phys_start(),
            arch::x86_64::reserved::kernel_phys_end(),
        ),
    ];
}

/// Subtracts one reserved half-open range from one usable half-open range.
///
/// The result may contain:
/// - no remaining usable range if the reserved span fully covers the input
/// - one remaining usable range for no overlap or edge overlap
/// - two remaining usable ranges if the reserved span splits the input
fn subtract_range(usable: PhysicalRange, reserved: PhysicalRange) -> [Option<PhysicalRange>; 2] {
    let usable_start = usable.start.as_u64();
    let usable_end = usable.end_exclusive.as_u64();
    let reserved_start = reserved.start.as_u64();
    let reserved_end = reserved.end_exclusive.as_u64();

    if reserved_end <= usable_start || reserved_start >= usable_end {
        return [Some(usable), None];
    }

    let overlap_start = if reserved_start > usable_start {
        reserved_start
    } else {
        usable_start
    };
    let overlap_end = if reserved_end < usable_end {
        reserved_end
    } else {
        usable_end
    };

    if overlap_start <= usable_start && overlap_end >= usable_end {
        return [None, None];
    }

    if overlap_start <= usable_start {
        return [
            Some(physical_range(
                PhysicalAddress::new(overlap_end),
                usable.end_exclusive,
            )),
            None,
        ];
    }

    if overlap_end >= usable_end {
        return [
            Some(physical_range(
                usable.start,
                PhysicalAddress::new(overlap_start),
            )),
            None,
        ];
    }

    return [
        Some(physical_range(
            usable.start,
            PhysicalAddress::new(overlap_start),
        )),
        Some(physical_range(
            PhysicalAddress::new(overlap_end),
            usable.end_exclusive,
        )),
    ];
}

impl MonotonicFrameAllocator {
    /// Creates an empty allocator with no usable frame regions loaded yet.
    pub const fn empty() -> Self {
        Self {
            regions: [EMPTY_USABLE_FRAME_REGION; MAX_ALLOCATABLE_FRAME_REGIONS],
            region_count: 0,
            current_region: 0,
            next_frame: None,
            freed_frames: [None; MAX_ALLOCATABLE_FRAME_REGIONS],
            freed_count: 0,
        }
    }

    /// Returns whether `frame` lies inside one of the allocator-owned usable ranges.
    fn is_allocatable_frame(&self, frame: PhysicalFrame) -> bool {
        for region in self.regions.iter().take(self.region_count) {
            let frame_addr = frame.start_address().as_u64();
            let region_start = region.start.start_address().as_u64();
            let region_end = region.end_exclusive.start_address().as_u64();

            if frame_addr >= region_start && frame_addr < region_end {
                return true;
            }
        }

        return false;
    }

    /// Returns whether `frame` lies inside one of the permanently reserved spans.
    fn is_reserved_frame(frame: PhysicalFrame) -> bool {
        let frame_start = frame.start_address().as_u64();
        let frame_end = match frame_start.checked_add(FRAME_SIZE) {
            Some(value) => value,
            None => return false,
        };
        for reserved in collect_reserved_ranges() {
            let reserved_start = reserved.start.as_u64();
            let reserved_end = reserved.end_exclusive.as_u64();

            if frame_start < reserved_end && frame_end > reserved_start {
                return true;
            }
        }

        return false;
    }

    /// Returns whether the monotonic cursor has already handed out `frame` once.
    fn was_monotonically_allocated(&self, frame: PhysicalFrame) -> bool {
        if self.region_count == 0 {
            return false;
        }

        if self.current_region >= self.region_count || self.next_frame.is_none() {
            return self.is_allocatable_frame(frame);
        }

        let frame_addr = frame.start_address().as_u64();

        for region in self.regions.iter().take(self.current_region) {
            let region_start = region.start.start_address().as_u64();
            let region_end = region.end_exclusive.start_address().as_u64();

            if frame_addr >= region_start && frame_addr < region_end {
                return true;
            }
        }

        let current_region = self.regions[self.current_region];
        let current_region_start = current_region.start.start_address().as_u64();
        let next_frame_addr = self.next_frame.unwrap().start_address().as_u64();

        return frame_addr >= current_region_start && frame_addr < next_frame_addr;
    }

    /// Returns whether `frame` is already present in the freed-frame reuse stack.
    fn is_already_freed(&self, frame: PhysicalFrame) -> bool {
        for freed_frame in self.freed_frames.iter().take(self.freed_count).flatten() {
            if *freed_frame == frame {
                return true;
            }
        }

        return false;
    }

    /// Normalizes one remaining usable physical span and appends it if a full
    /// 4 KiB frame still fits after alignment.
    fn push_allocatable_range(&mut self, range: PhysicalRange) -> Result<(), AllocatorInitError> {
        let aligned_start = range
            .start
            .checked_align_up()
            .ok_or(AllocatorInitError::RegionOverflow)?;
        let aligned_end = range.end_exclusive.align_down();

        if aligned_start.as_u64() >= aligned_end.as_u64() {
            return Ok(());
        }

        let start = PhysicalFrame::from_start_address(aligned_start)
            .ok_or(AllocatorInitError::BadFrameRegion)?;
        let end_exclusive = PhysicalFrame::from_start_address(aligned_end)
            .ok_or(AllocatorInitError::BadFrameRegion)?;

        if self.region_count >= MAX_ALLOCATABLE_FRAME_REGIONS {
            return Err(AllocatorInitError::TooManyAllocatableRegions);
        }

        self.regions[self.region_count] = UsableFrameRegion {
            start,
            end_exclusive,
        };
        self.region_count += 1;
        Ok(())
    }

    /// Builds allocator-owned usable frame ranges from the parsed boot memory map.
    ///
    /// The constructor keeps only `Usable` regions, normalizes them to 4 KiB
    /// frame boundaries, discards post-alignment empty ranges, resets the
    /// freed-frame reuse stack, and initializes the allocator cursor to the
    /// first allocatable frame if one exists.
    pub fn initialize_from_boot_info(
        &mut self,
        boot_info: &BootInfo,
    ) -> Result<(), AllocatorInitError> {
        let reserved_ranges = collect_reserved_ranges();

        self.regions = [EMPTY_USABLE_FRAME_REGION; MAX_ALLOCATABLE_FRAME_REGIONS];
        self.region_count = 0;
        self.current_region = 0;
        self.next_frame = None;
        self.freed_frames = [None; MAX_ALLOCATABLE_FRAME_REGIONS];
        self.freed_count = 0;

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
            let mut fragments: [Option<PhysicalRange>; MAX_REGION_FRAGMENTS] =
                [Some(physical_range(raw_start, raw_end)), None, None];

            for reserved in reserved_ranges {
                let mut next_fragments: [Option<PhysicalRange>; MAX_REGION_FRAGMENTS] =
                    [None; MAX_REGION_FRAGMENTS];
                let mut next_count = 0;

                for fragment in fragments.iter().flatten() {
                    for piece in subtract_range(*fragment, reserved).iter().flatten() {
                        if next_count >= MAX_REGION_FRAGMENTS {
                            return Err(AllocatorInitError::TooManyAllocatableRegions);
                        }

                        next_fragments[next_count] = Some(*piece);
                        next_count += 1;
                    }
                }

                fragments = next_fragments;
            }

            for fragment in fragments.iter().flatten() {
                self.push_allocatable_range(*fragment)?;
            }
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

    /// Returns the next allocatable 4 KiB frame.
    ///
    /// Freed frames are reused first in LIFO order. If no freed frame is
    /// available, allocation falls back to the monotonic usable-region cursor.
    pub fn allocate_frame(&mut self) -> Option<PhysicalFrame> {
        if self.freed_count != 0 {
            self.freed_count -= 1;
            let freed_frame = self.freed_frames[self.freed_count].take();
            return freed_frame;
        }

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

    /// Returns one runtime-owned frame to the allocator's LIFO reuse stack.
    ///
    /// This rejects permanently reserved frames, unmanaged frames, and frames
    /// that are already waiting in the reuse stack.
    pub fn free(&mut self, frame: PhysicalFrame) -> Result<(), FreeError> {
        if Self::is_reserved_frame(frame) {
            return Err(FreeError::ReservedFrame);
        }

        if !self.is_allocatable_frame(frame) || !self.was_monotonically_allocated(frame) {
            return Err(FreeError::UnmanagedFrame);
        }

        if self.is_already_freed(frame) {
            return Err(FreeError::FrameAlreadyFreed);
        }

        if self.freed_count >= MAX_ALLOCATABLE_FRAME_REGIONS {
            return Err(FreeError::FreedFrameOverflow);
        }

        self.freed_frames[self.freed_count] = Some(frame);
        self.freed_count += 1;
        return Ok(());
    }
}
