use core::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

/// One minimal interrupt-safe spinlock for early kernel shared state.
pub struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

/// One held spinlock guard that restores the prior interrupt state on drop.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
    interrupts_were_enabled: bool,
}

// Sound because `SpinLock` serializes all mutable access to `value`, and the
// protected payload may move across threads/CPUs only when `T` itself is `Send`.
unsafe impl<T: Send> Sync for SpinLock<T> {}
// Sound because moving the lock between threads/CPUs is safe whenever the
// protected payload itself may be transferred.
unsafe impl<T: Send> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    /// Builds one spinlock around `value`.
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    /// Acquires the lock while preserving the caller's prior interrupt state.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        let interrupts_were_enabled = interrupts_enabled();
        disable_interrupts();

        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }

        SpinLockGuard {
            lock: self,
            interrupts_were_enabled,
        }
    }
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Sound because holding the guard proves exclusive lock ownership, so
        // creating one shared reference to the protected payload is valid.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Sound because holding the guard proves exclusive lock ownership, so
        // creating one mutable reference to the protected payload is valid.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        if self.interrupts_were_enabled {
            enable_interrupts();
        }
    }
}

fn interrupts_enabled() -> bool {
    let rflags: u64;

    // Sound because reading `rflags` through `pushfq`/`pop` observes only the
    // current interrupt-enable bit and does not mutate memory.
    unsafe {
        core::arch::asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem, preserves_flags),
        );
    }

    return rflags & (1 << 9) != 0;
}

fn disable_interrupts() {
    // Sound because the caller uses this only to establish an interrupt-free
    // critical section around early-kernel shared-state access.
    unsafe { core::arch::asm!("cli", options(nomem, nostack, preserves_flags)) }
}

fn enable_interrupts() {
    // Sound because the guard restores only a previously enabled interrupt
    // state after releasing the lock.
    unsafe { core::arch::asm!("sti", options(nomem, nostack, preserves_flags)) }
}
