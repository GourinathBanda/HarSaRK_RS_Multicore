use crate::kernel::tasks::{TaskManager, TaskManager_C1, schedule};
use crate::system::resource::{PiStackGlobal, PiStackGlobal_C1, Resource};
use crate::system::scheduler::{BooleanVector, Scheduler};
use crate::utils::arch::{critical_section, Mutex};
use crate::system::spinlock::{spinlock, spinlock_try, spinunlock, TASKMANAGER_LOCK};
use crate::KernelError;

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::RefCell;
use cortex_m_semihosting::hprintln;

/// this spinlock is used to synchronize access of `TaskManager`s across cores. The reason for
/// using spin lock in this file instead of making the mutex a spinlock mutex is that the
/// bare_metal::Mutex has qualities like depending on CriticalSection and being deadlock free.
/// Also spinlock mutex will introduce unnecessary overheads.
use cortex_m::interrupt::CriticalSection;

/// A Shared container for resources to be shared across multiple cores
pub struct Shared<'a, T: Sized> {
    resource: Resource<T>,
    // TODO: this has to be static because Resource contains a reference to TaskManager which is
    // static? anyway try to fix this
    lock_ref: &'a AtomicBool,
    curr_tid_ref: &'a RefCell<usize>,
    other_resource_taskmask: BooleanVector,
    other_core_task_manager: &'static Mutex<RefCell<Scheduler>>,
}

impl<'a, T: Sized> Shared<'a, T> {
    pub const fn new(
        resource: Resource<T>,
        lock_ref: &'a AtomicBool,
        other_resource_taskmask: BooleanVector,
        other_core_task_manager: &'static Mutex<RefCell<Scheduler>>,
        curr_tid_ref: &'a RefCell<usize>,
    ) -> Self {
        Shared { resource, lock_ref, other_resource_taskmask, other_core_task_manager, curr_tid_ref }
    }

    pub fn lock(&self) -> Result<&T, KernelError> {
        let v = self.resource.lock()?;
        // spin lock here
        // TODO: make a usable spinlock api
        while let Err(_) =
            self.lock_ref
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            // check if the task running on the other core is the same one which has locked the
            // resource. To check this, it is enough to check
            // `other_core_resource.task_mask & other_core_task_manager.curr_tid != 0`
            critical_section(|cs_token| {
                if let Ok(_) = spinlock_try(&TASKMANAGER_LOCK)
                {
                    let oc_crr_tid = self.other_core_task_manager.borrow(cs_token).borrow().curr_tid;

                    hprintln!("spin: oc={:b}, oc_taskmask={:b}, res={}", 1 << oc_crr_tid as u32 , self.other_resource_taskmask, oc_crr_tid as u32 & self.other_resource_taskmask);
                    if ((1 << oc_crr_tid as u32) & self.other_resource_taskmask) == 0 {
                        // hprintln!("migration set: oc={:b}, oc_taskmask={:b}, res={}", oc_crr_tid as u32, self.other_resource_taskmask, oc_crr_tid as u32 & self.other_resource_taskmask);
                        // this means that the task executing on the other core is not the one that
                        // locked the resource. in other words, the resource that has locked the
                        // resource has been preempted.
                        let migrated_tid = *self.curr_tid_ref.borrow();
                        let mut oc_handler = self.other_core_task_manager.borrow(cs_token).borrow_mut();
                        oc_handler.migrated_tasks |= (1 << migrated_tid);

                        let mut handler = self.resource.task_manager.borrow(cs_token).borrow_mut();
                        handler.migrated_tid = migrated_tid;
                    }
                    spinunlock(&TASKMANAGER_LOCK);
                    schedule(self.resource.task_manager);
                }
            });

        }
        critical_section(|cs_token| {
            spinlock(&TASKMANAGER_LOCK);
            let mut tid = self.resource.task_manager.borrow(cs_token).borrow().curr_tid;
            *self.curr_tid_ref.borrow_mut() = tid;
            spinunlock(&TASKMANAGER_LOCK);
        });
        Ok(v)
    }

    pub fn unlock(&self) -> Result<(), KernelError> {
        self.lock_ref.store(false, Ordering::SeqCst);
        self.resource.unlock();
        Ok(())
    }

    /// A helper function that ensures that if a resource is locked, it is unlocked.
    pub fn acquire<F, R>(&self, handler: F) -> Result<R, KernelError>
    where
        F: Fn(&T) -> R,
    {
        let value = self.lock()?;
        let res = handler(value);
        self.unlock()?;
        return Ok(res);
    }
}

pub struct SharedResource<T: Sized> {
    val: T,
    tasks_mask0: BooleanVector,
    tasks_mask1: BooleanVector,
    lock: AtomicBool,
    curr_tid: RefCell<usize>,
}

impl<T: Sized> SharedResource<T> {
    /// tasks_mask0 is the task mask of this reosource for core 0
    /// tasks_mask1 is the task mask of this reosource for core 1
    pub const fn new(val: T, tasks_mask0: BooleanVector, tasks_mask1: BooleanVector) -> Self {
        Self { val, tasks_mask0, tasks_mask1, lock: AtomicBool::new(false), curr_tid: RefCell::new(0) }
    }

    pub fn core0(&self) -> Shared<&T> {
        Shared::new(
            Resource::new(&TaskManager, &PiStackGlobal, &self.val, self.tasks_mask0),
            &self.lock,
            self.tasks_mask1,
            &TaskManager_C1,
            &self.curr_tid,
        )
    }

    pub fn core1(&self) -> Shared<&T> {
        Shared::new(
            Resource::new(&TaskManager_C1, &PiStackGlobal_C1, &self.val, self.tasks_mask1),
            &self.lock,
            self.tasks_mask1,
            &TaskManager,
            &self.curr_tid,
        )
    }
}
unsafe impl<T> Sync for SharedResource<T> {}
unsafe impl<T> Sync for Shared<'_, T> {}

// fn spin_criticalsection<F, R>(f: F) -> R
// where
//     F: FnOnce(&CriticalSection) -> R,
// {
//     while let Err(_) = SPINLOCK.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {}
//     let t = critical_section(f);
//     SPINLOCK.store(false, Ordering::SeqCst);
//     t
// }
