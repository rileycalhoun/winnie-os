#[cfg(test)]
mod mem_tests {
    use core::cell::UnsafeCell;

    use winnie_os::{
        arch::x86_64::reserved,
        boot_info::{BootInfo, MemoryRegion, MemoryRegionKind},
        memory::{FRAME_SIZE, PhysicalAddress, PhysicalFrame, allocator::MonotonicFrameAllocator},
    };

    fn region(base: u64, length: u64, kind: MemoryRegionKind) -> MemoryRegion {
        MemoryRegion { base, length, kind }
    }

    /// Verifies the public memory module exports support aligned frame construction.
    #[test_case]
    fn exported_frame_types_support_aligned_frame_construction() {
        let addr = PhysicalAddress::new(0x2000);
        let frame = PhysicalFrame::from_start_address(addr);

        assert!(frame.is_some());
    }

    struct TestBootInfoStorage(UnsafeCell<BootInfo>);
    // Sound because the bootable kernel test harness runs tests serially during
    // early single-threaded bring-up, so this storage is mutably borrowed by
    // only one test at a time.
    unsafe impl Sync for TestBootInfoStorage {}
    static TEST_BOOT_INFO: TestBootInfoStorage =
        TestBootInfoStorage(UnsafeCell::new(BootInfo::new()));

    struct TestMonotonicAllocator(UnsafeCell<MonotonicFrameAllocator>);
    // Sound because the bootable kernel test harness runs tests serially during
    // early single-threaded bring-up, so this storage is mutably borrowed by
    // only one test at a time.
    unsafe impl Sync for TestMonotonicAllocator {}
    static TEST_MONOTONIC_ALLOCATOR: TestMonotonicAllocator =
        TestMonotonicAllocator(UnsafeCell::new(MonotonicFrameAllocator::empty()));

    #[test_case]
    fn monotonic_allocator_skips_reserved_regions_and_exhausts() {
        // Sound because the bootable kernel test harness executes tests
        // serially, and this fixture storage is reused by only one test at a
        // time.
        let boot_info = unsafe { &mut *TEST_BOOT_INFO.0.get() };
        *boot_info = BootInfo::new();
        boot_info
            .push_region(region(0x0000, 0x1000, MemoryRegionKind::Reserved))
            .unwrap();

        boot_info
            .push_region(region(0x1000, 0x2000, MemoryRegionKind::Usable))
            .unwrap();

        boot_info
            .push_region(region(0x3000, 0x2000, MemoryRegionKind::Reserved))
            .unwrap();

        boot_info
            .push_region(region(0x5000, 0x2000, MemoryRegionKind::Usable))
            .unwrap();

        // Sound because the bootable kernel test harness executes tests
        // serially, and this fixture storage is reused by only one test at a
        // time.
        let allocator = unsafe { &mut *TEST_MONOTONIC_ALLOCATOR.0.get() };
        allocator.initialize_from_boot_info(boot_info).unwrap();

        assert_eq!(
            allocator.allocate_frame().unwrap().start_address(),
            PhysicalAddress::new(0x1000)
        );
        assert_eq!(
            allocator.allocate_frame().unwrap().start_address(),
            PhysicalAddress::new(0x2000)
        );
        assert_eq!(
            allocator.allocate_frame().unwrap().start_address(),
            PhysicalAddress::new(0x5000)
        );
        assert_eq!(
            allocator.allocate_frame().unwrap().start_address(),
            PhysicalAddress::new(0x6000)
        );
        assert!(allocator.allocate_frame().is_none());
    }

    /// Verifies the allocator skips the real exported bootstrap and kernel-image spans.
    #[test_case]
    fn monotonic_allocator_skips_kernel_owned_reserved_ranges() {
        let reserved_start = reserved::boot_phys_start().as_u64();
        let reserved_end = reserved::kernel_phys_end().as_u64();
        let first_frame_after_reserved = PhysicalAddress::new(reserved_end)
            .checked_align_up()
            .unwrap();
        let usable_start = reserved_start.checked_sub(FRAME_SIZE).unwrap();
        let usable_end = first_frame_after_reserved
            .as_u64()
            .checked_add(FRAME_SIZE)
            .unwrap();

        // Sound because the bootable kernel test harness executes tests
        // serially, and this fixture storage is reused by only one test at a
        // time.
        let boot_info = unsafe { &mut *TEST_BOOT_INFO.0.get() };
        *boot_info = BootInfo::new();
        boot_info
            .push_region(region(
                usable_start,
                usable_end - usable_start,
                MemoryRegionKind::Usable,
            ))
            .unwrap();

        // Sound because the bootable kernel test harness executes tests
        // serially, and this fixture storage is reused by only one test at a
        // time.
        let allocator = unsafe { &mut *TEST_MONOTONIC_ALLOCATOR.0.get() };
        allocator.initialize_from_boot_info(boot_info).unwrap();

        assert_eq!(
            allocator.allocate_frame().unwrap().start_address(),
            PhysicalAddress::new(usable_start)
        );
        assert_eq!(
            allocator.allocate_frame().unwrap().start_address(),
            first_frame_after_reserved
        );
        assert!(allocator.allocate_frame().is_none());
    }
}
