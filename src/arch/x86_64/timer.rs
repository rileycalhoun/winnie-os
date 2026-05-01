use core::ptr;

use crate::{
    arch::x86_64::{apic, idt::InterruptFrame, paging},
    println,
    sync::spinlock::SpinLock,
};

/// First external IRQ vector reserved for the LAPIC timer interrupt.
///
/// This stays above the CPU exception range and inside the conventional
/// remapped-IRQ window so future external interrupts can remain clearly
/// separated from synchronous CPU faults.
pub const TIMER_IRQ_VECTOR: u8 = 32;

/// One-shot marker emitted when the timer proof path has observed a real tick.
pub const TIMER_PROOF_MARKER: &str = "TIMER PROOF OK";
/// Serial marker emitted once the LAPIC timer has been configured.
pub const TIMER_INIT_MARKER: &str = "TIMER INIT OK";

const LAPIC_TIMER_LVT_OFFSET: u64 = 0x320;
const LAPIC_TIMER_INITIAL_COUNT_OFFSET: u64 = 0x380;
const LAPIC_TIMER_DIVIDE_CONFIGURATION_OFFSET: u64 = 0x3e0;

const LAPIC_TIMER_PERIODIC_MODE_BIT: u32 = 1 << 17;
const LAPIC_TIMER_DIVIDE_BY_16: u32 = 0x3;
const LAPIC_TIMER_INITIAL_COUNT: u32 = 100_000;

struct TimerState {
    tick_count: u64,
    first_tick_observed: bool,
}

static TIMER_STATE: SpinLock<TimerState> = SpinLock::new(TimerState {
    tick_count: 0,
    first_tick_observed: false,
});

/// Reports failure while configuring the LAPIC timer path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerInitError {
    LapicNotInitialized,
}

/// Resets the shared timer proof state before a fresh normal-boot run.
pub fn reset_timer_state() {
    let mut timer_state = TIMER_STATE.lock();
    timer_state.tick_count = 0;
    timer_state.first_tick_observed = false;
}

/// Returns the total number of timer ticks observed so far.
pub fn tick_count() -> u64 {
    TIMER_STATE.lock().tick_count
}

/// Returns whether the timer path has observed at least one real interrupt.
pub fn first_tick_observed() -> bool {
    TIMER_STATE.lock().first_tick_observed
}

/// Records one timer interrupt and returns the resulting total tick count.
pub fn record_timer_tick() -> u64 {
    let mut timer_state = TIMER_STATE.lock();
    timer_state.tick_count += 1;
    timer_state.first_tick_observed = true;
    timer_state.tick_count
}

/// Programs the LAPIC timer in periodic mode on the chosen timer IRQ vector.
pub fn initialize() -> Result<(), TimerInitError> {
    if !apic::is_initialized() {
        return Err(TimerInitError::LapicNotInitialized);
    }

    write_register(
        LAPIC_TIMER_LVT_OFFSET,
        TIMER_IRQ_VECTOR as u32 | LAPIC_TIMER_PERIODIC_MODE_BIT,
    );
    write_register(
        LAPIC_TIMER_DIVIDE_CONFIGURATION_OFFSET,
        LAPIC_TIMER_DIVIDE_BY_16,
    );
    write_register(LAPIC_TIMER_INITIAL_COUNT_OFFSET, LAPIC_TIMER_INITIAL_COUNT);
    Ok(())
}

/// Handles the LAPIC timer IRQ path.
pub extern "x86-interrupt" fn timer_interrupt_handler(_frame: &InterruptFrame) {
    let tick_count = record_timer_tick();
    if tick_count == 1 {
        println!("{}", TIMER_PROOF_MARKER);
    }

    apic::end_of_interrupt();
}

fn register_ptr(offset: u64) -> *mut u32 {
    return (paging::RUNTIME_MMIO_PAGE_VIRT_ADDR + offset) as *mut u32;
}

fn write_register(offset: u64, value: u32) {
    let register = register_ptr(offset);

    // Sound because Task 3 maps the LAPIC page at the fixed runtime MMIO slot
    // before timer initialization, and these offsets target architecturally
    // defined 32-bit LAPIC timer registers within that page.
    unsafe { ptr::write_volatile(register, value) }
}
