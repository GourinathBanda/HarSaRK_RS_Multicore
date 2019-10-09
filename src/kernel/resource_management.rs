use crate::process::get_pid;
use crate::config::MAX_RESOURCES;
use crate::errors::KernelError;
use crate::kernel::helper::get_msb;
use crate::process::{block_tasks, schedule, unblock_tasks};
use core::cmp::max;
use core::pin::Pin;
use cortex_m_semihosting::hprintln;

use crate::kernel::types::ResourceId;

const PI: u32 = 0;

#[derive(Clone, Copy)]
pub struct ResourceControlBlock {
    rt_ceiling: u32,
    tasks_mask: u32
}

#[derive(Clone, Copy)]
pub struct ResourceManager {
    resources_block: [ResourceControlBlock; MAX_RESOURCES], // Resource Control Block, holds u32 expressing which tasks have access to it.
    top: usize,
    pi_stack: [u32; MAX_RESOURCES],
    curr: usize, // used to track current no. of resources initialized
    system_ceiling: u32,
}

impl ResourceControlBlock {
    pub const fn new() -> Self {
        Self {
            rt_ceiling: PI,
            tasks_mask: PI
        }
    }
    pub fn set(&mut self, tasks_mask: u32) {
        self.tasks_mask = tasks_mask;
        self.rt_ceiling = get_msb(tasks_mask) as u32;
    }
}

impl ResourceManager {
    pub const fn new() -> Self {
        ResourceManager {
            resources_block: [ResourceControlBlock::new(); MAX_RESOURCES],
            top: 0,
            pi_stack: [0; MAX_RESOURCES],
            curr: 0,
            system_ceiling: PI,
        }
    }

    pub fn create(&mut self, tasks_mask: u32) -> Result<ResourceId, KernelError> {
        let id = self.curr;
        if id >= MAX_RESOURCES {
            return Err(KernelError::LimitExceeded);
        }
        self.resources_block[id].set(tasks_mask);
        self.curr += 1;
        Ok(id)
    }

    pub fn lock(&mut self, id: ResourceId) -> bool {
        let resource = &self.resources_block[id];
        let curr_pid = get_pid();
        let rt_ceiling = resource.rt_ceiling;

        let pid_mask = 1<<curr_pid;
        
        if resource.tasks_mask & pid_mask != pid_mask {
            return false
        }

        if rt_ceiling > self.system_ceiling {
            self.push_stack(rt_ceiling);

            let mut mask = 1<<(rt_ceiling+1) - 1;
            mask &= !(1<<curr_pid);
        
            self.system_ceiling = self.resources_block[id].rt_ceiling;
            block_tasks(mask);
            return true;
        }
        return false;
    }

    pub fn unlock(&mut self, id: ResourceId) {
        let resource = self.resources_block[id];
        if resource.rt_ceiling == self.system_ceiling {
            self.pop_stack();
            let mut mask = 1<<(resource.rt_ceiling+1) - 1;
            unblock_tasks(mask);
            schedule();
        }
    }

    fn pop_stack(&mut self) {
        self.system_ceiling = self.pi_stack[self.top - 1];
        self.top -= 1;
    }

    fn push_stack(&mut self, ceiling: u32) {
        self.pi_stack[self.top] = self.system_ceiling;
        self.top += 1;
    }
}
