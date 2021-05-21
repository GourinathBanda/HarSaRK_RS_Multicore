use core::sync::atomic::{AtomicBool, Ordering};

pub static TASKMANAGER_LOCK: AtomicBool = AtomicBool::new(false);

pub fn spinlock_try<'a>(lock: &'a AtomicBool) -> Result<bool, bool> {
    lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
}

#[inline(never)]
pub fn spinlock<'a>(lock: &'a AtomicBool) {
    while let Err(_) =
        lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            // do nothing
        }
}

#[inline(never)]
pub fn spinunlock<'a>(lock: &'a AtomicBool) {
    lock.store(false, Ordering::SeqCst);
}

// TODO: refactor spinlock that accepts a closure. but this might increase the code size, is it
// worth it?
