use core::sync::atomic::{AtomicBool, Ordering};

pub static TASKMANAGER_LOCK: AtomicBool = AtomicBool::new(false);

pub fn spinlock_try<'a>(lock: &'a AtomicBool) -> Result<bool, bool> {
    lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
}

pub fn spinlock<'a>(lock: &'a AtomicBool) {
    while let Err(_) =
        lock.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            // do nothing
        }
}

pub fn spinunlock<'a>(lock: &'a AtomicBool) {
    lock.store(false, Ordering::SeqCst);
}
