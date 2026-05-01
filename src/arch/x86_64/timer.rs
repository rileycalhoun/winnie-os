use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// First external IRQ vector reserved for the LAPIC timer interrupt.
///
/// This stays above the CPU exception range and inside the conventional
/// remapped-IRQ window so future external interrupts can remain clearly
/// separated from synchronous CPU faults.
pub const TIMER_IRQ_VECTOR: u8 = 32;

/// One-shot marker emitted when the timer proof path has observed a real tick.
pub const TIMER_PROOF_MARKER: &str = "TIMER PROOF OK";

static TIMER_TICK_COUNT: AtomicU64 = AtomicU64::new(0);
static FIRST_TICK_OBSERVED: AtomicBool = AtomicBool::new(false);

/// Resets the shared timer proof state before a fresh normal-boot run.
pub fn reset_timer_state() {
    TIMER_TICK_COUNT.store(0, Ordering::Relaxed);
    FIRST_TICK_OBSERVED.store(false, Ordering::Relaxed);
}

/// Returns the total number of timer ticks observed so far.
pub fn tick_count() -> u64 {
    TIMER_TICK_COUNT.load(Ordering::Relaxed)
}

/// Returns whether the timer path has observed at least one real interrupt.
pub fn first_tick_observed() -> bool {
    FIRST_TICK_OBSERVED.load(Ordering::Relaxed)
}

/// Records one timer interrupt and returns the resulting total tick count.
pub fn record_timer_tick() -> u64 {
    let tick_count = TIMER_TICK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    FIRST_TICK_OBSERVED.store(true, Ordering::Relaxed);
    tick_count
}
