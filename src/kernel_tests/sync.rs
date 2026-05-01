#[cfg(test)]
mod sync_tests {
    use winnie_os::sync::spinlock::SpinLock;

    /// Verifies the minimal spinlock API can protect one shared scalar value.
    #[test_case]
    fn spinlock_guards_one_shared_value() {
        let lock = SpinLock::new(41_u64);

        {
            let mut guard = lock.lock();
            *guard += 1;
        }

        let guard = lock.lock();
        assert_eq!(*guard, 42);
    }
}
