/// The fixed 4 KiB frame size currently used by Winnie OS physical memory code.
pub const FRAME_SIZE: u64 = 4096;
const FRAME_MASK: u64 = FRAME_SIZE - 1;

/// A raw physical address value used by early frame-allocation and mapping code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const fn new(addr: u64) -> Self {
        return Self(addr);
    }

    pub const fn as_u64(self) -> u64 {
        return self.0;
    }

    pub const fn is_aligned(self) -> bool {
        return self.0 & FRAME_MASK == 0;
    }

    pub const fn align_down(self) -> Self {
        return Self(self.0 & !FRAME_MASK);
    }

    pub const fn align_up(self) -> Self {
        match self.checked_align_up() {
            Some(addr) => return addr,
            None => panic!("physical address alignment overflow"),
        }
    }

    /// Rounds this address up to the next 4 KiB boundary if one exists.
    pub const fn checked_align_up(self) -> Option<Self> {
        if self.is_aligned() {
            return Some(self);
        }

        return match self.0.checked_add(FRAME_MASK) {
            Some(value) => Some(Self(value & !FRAME_MASK)),
            None => None,
        };
    }
}

/// One 4 KiB physical frame identified by its aligned starting address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalFrame {
    start: PhysicalAddress,
}

impl PhysicalFrame {
    /// Builds a physical frame only if `addr` is already aligned to 4 KiB.
    pub const fn from_start_address(addr: PhysicalAddress) -> Option<Self> {
        if addr.is_aligned() {
            return Some(Self { start: addr });
        } else {
            return None;
        }
    }

    pub const fn start_address(self) -> PhysicalAddress {
        return self.start;
    }

    pub const fn contains_address(self, addr: PhysicalAddress) -> bool {
        let start = self.start.as_u64();
        let value = addr.as_u64();

        return match start.checked_add(FRAME_SIZE) {
            Some(end) => value >= start && value < end,
            None => false,
        };
    }

    /// Advances to the next 4 KiB physical frame if the address does not overflow.
    pub const fn checked_next(self) -> Option<Self> {
        return match self.start.as_u64().checked_add(FRAME_SIZE) {
            Some(next_start) => Some(Self {
                start: PhysicalAddress::new(next_start),
            }),
            None => None,
        };
    }

    pub const fn next(self) -> Self {
        match self.checked_next() {
            Some(next) => next,
            None => panic!("physical frame advance overflow"),
        }
    }
}
