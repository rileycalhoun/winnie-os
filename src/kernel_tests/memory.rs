#[cfg(test)]
mod mem_tests {
    use winnie_os::memory::{PhysicalAddress, PhysicalFrame};

    /// Verifies the public memory module exports support aligned frame construction.
    #[test_case]
    fn exported_frame_types_support_aligned_frame_construction() {
        let addr = PhysicalAddress::new(0x2000);
        let frame = PhysicalFrame::from_start_address(addr);

        assert!(frame.is_some());
    }
}
