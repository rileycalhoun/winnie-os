//! Physical-memory address and frame primitives for early runtime ownership.
pub use frame::{FRAME_SIZE, PhysicalAddress, PhysicalFrame};

pub mod allocator;
pub mod frame;
