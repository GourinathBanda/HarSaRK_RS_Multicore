
use crate::system::resource::{Resource, PiStackGlobal, PiStackGlobal_C1};
use crate::system::scheduler::BooleanVector;
use crate::kernel::tasks::{TaskManager, TaskManager_C1};
use crate::KernelError;

use core::sync::atomic::{AtomicBool, Ordering};
use cortex_m_semihosting::hprint;


/// A Shared container for resources to be shared across multiple cores
pub struct Shared<'a, T: Sized> {
    resource: Resource<T>,
    // TODO: this has to be static because Resource contains a reference to TaskManager which is
    // static? anyway try to fix this
    lock_ref: &'a AtomicBool,
}

impl<'a, T: Sized> Shared<'a, T> {
    pub const fn new(resource: Resource<T>, lock_ref: &'a AtomicBool) -> Self {
        Shared { resource, lock_ref }
    }

    pub fn lock(&self) -> Result<&T, KernelError> {
        let v = self.resource.lock()?;
        // spin lock here
        while let Err(_) = self.lock_ref.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        {
            hprint!("spin... ");
            // check if the task running on the other core is the same one which has locked the
            // resource. To check this, it is enough to check
            // `other_core_resource.task_mask & other_core_task_manager.curr_tid != 0`
        }
        Ok(v)
    }

    pub fn unlock(&self) -> Result<(), KernelError> {
        self.resource.unlock();
        self.lock_ref.store(false, Ordering::SeqCst);
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
}

impl<T: Sized> SharedResource<T> {
    /// tasks_mask0 is the task mask of this reosource for core 0
    /// tasks_mask1 is the task mask of this reosource for core 1
    pub const fn new(val: T, tasks_mask0: BooleanVector, tasks_mask1: BooleanVector) -> Self {
        Self { val, tasks_mask0, tasks_mask1, lock: AtomicBool::new(false) }
    }

    pub fn core0(&self) -> Shared<&T> {
        Shared::new(Resource::new(&TaskManager, &PiStackGlobal, &self.val, self.tasks_mask0), &self.lock)
    }

    pub fn core1(&self) -> Shared<&T> {
        Shared::new(Resource::new(&TaskManager_C1, &PiStackGlobal_C1, &self.val, self.tasks_mask1), &self.lock)
    }
}
