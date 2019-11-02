use crate::config::MAX_TASKS;
use core::cell::{RefCell};
use cortex_m::interrupt::free as execute_critical;
use cortex_m::interrupt::Mutex;

use crate::errors::KernelError;
use crate::internals::helper::check_priv;
use crate::internals::resource_manager::ResourceManager;
use crate::internals::types::ResourceId;
use crate::priv_execute;

use crate::process::{block_tasks, get_pid, schedule, unblock_tasks};



use cortex_m::register::control::Npriv;

static resources_list: Mutex<RefCell<ResourceManager>> =
    Mutex::new(RefCell::new(ResourceManager::new()));

#[derive(Debug)]
pub struct Resource<T: Sized> {
    inner: T,
    id: ResourceId,
}

impl<T> Resource<T> {
    pub fn new(val: T, id: ResourceId) -> Self {
        Self { inner: val, id }
    }

    fn lock(&self) -> Option<&T> {
        execute_critical(|cs_token| {
            let pid = match check_priv() {
                Npriv::Privileged => 1,
                Npriv::Unprivileged => get_pid() as u32
            };
            let res = resources_list
                .borrow(cs_token)
                .borrow_mut()
                .lock(self.id, pid);
            if let Some(mask) = res {
                block_tasks(mask);
                return Some(&self.inner);
            }
            return None;
        })
    }

    fn unlock(&self) {
        execute_critical(|cs_token| {
            if let Some(mask) = resources_list.borrow(cs_token).borrow_mut().unlock(self.id) {
                unblock_tasks(mask);
                schedule();
            }
        })
    }

    pub fn acquire<F>(&self, handler: F)
    where
        F: Fn(&T),
    {
        if let Some(res) = self.lock() {
            handler(res);
            self.unlock();
        }
    }

    // only Privileged.
    pub unsafe fn access(&self) -> Result<&T, KernelError> {
        priv_execute!({ Ok(&self.inner) })
    }
}

pub fn create<T: Sized>(resource: T, tasks_mask: u32) -> Result<Resource<T>, KernelError> {
    // External interrupts and Privileged tasks have a priority of 0
    let tasks_mask = tasks_mask | 1<<0;
    priv_execute!({
        execute_critical(|cs_token| {
            let id = resources_list
                .borrow(cs_token)
                .borrow_mut()
                .create(tasks_mask)?;
            Ok(Resource::new(resource, id))
        })
    })
}

pub fn init_peripherals() -> Result<Resource<RefCell<cortex_m::Peripherals>>, KernelError> {
    let mut mask: u32 = 0;
    for i in 0..MAX_TASKS {
        mask |= 1 << i;
    }
    create(RefCell::new(cortex_m::Peripherals::take().unwrap()), mask)
}

unsafe impl<T> Sync for Resource<T> {}