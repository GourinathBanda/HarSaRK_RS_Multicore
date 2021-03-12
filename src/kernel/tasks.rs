//! # Task Management module
//! Defines Kernel routines which will take care of Task management functionality.
//! Declares a global instance of Scheduler that will be used by the Kernel routines to provide the functionality.

use core::cell::RefCell;

use crate::priv_execute;
use crate::system::scheduler::*;
use crate::utils::arch::is_privileged;
use crate::utils::arch::{critical_section, set_pendsv, svc_call, Mutex};
use crate::system::spinlock::{spinlock, spinlock_try, spinunlock, TASKMANAGER_LOCK};
use crate::KernelError;

#[cfg(feature = "system_logger")]
use crate::kernel::logging;
#[cfg(feature = "system_logger")]
use crate::system::system_logger::LogEventType;

/// Global Scheduler instance
#[no_mangle]
pub static TaskManager: Mutex<RefCell<Scheduler>> = Mutex::new(RefCell::new(Scheduler::new()));
#[no_mangle]
pub static TaskManager_C1: Mutex<RefCell<Scheduler>> = Mutex::new(RefCell::new(Scheduler::new()));

/// Initializes the Kernel scheduler and creates the idle task, a task that puts the CPU to sleep in a loop.
/// The idle task is created with zero priority; hence, it is only executed when no other task is in Ready state.
pub fn init(task_manager: &'static Mutex<RefCell<Scheduler>>, mut stack: &mut [u32]) -> Result<(), KernelError> {
    critical_section(|cs_token| task_manager.borrow(cs_token).borrow_mut().init(&mut stack))
}

/// Starts the Kernel scheduler, which starts scheduling tasks on the CPU.
pub fn start_kernel(task_manager: &'static Mutex<RefCell<Scheduler>>) -> ! {
    loop {
        schedule(task_manager);
    }
}

#[cfg(feature = "task_monitor")]
/// Create a new task with the configuration set as arguments passed.
pub fn create_task(
    priority: TaskId,
    deadline: u32,
    stack: &mut [u32],
    handler_fn: fn() -> !,
) -> Result<(), KernelError> {
    priv_execute!({
        critical_section(|cs_token| {
            TaskManager.borrow(cs_token).borrow_mut().create_task(
                priority as usize,
                deadline,
                stack,
                handler_fn,
            )
        })
    })
}

#[cfg(not(feature = "task_monitor"))]
/// Create a new task with the configuration set as arguments passed.
pub fn create_task(
    task_manager: &'static Mutex<RefCell<Scheduler>>,
    priority: TaskId,
    stack: &mut [u32],
    handler_fn: fn() -> !,
) -> Result<(), KernelError> {
    priv_execute!({
        critical_section(|cs_token| {
            task_manager.borrow(cs_token).borrow_mut().create_task(
                priority as usize,
                stack,
                handler_fn,
            )
        })
    })
}

/// This function is called from both privileged and unprivileged context.
/// Hence if the function is called from privileged context, then `preempt()` is called.
/// Else, the `svc_call()` is executed, this function creates the SVC exception.
/// And the SVC handler calls schedule again. Thus, the permission level is raised to privileged via the exception.
pub fn schedule(task_manager: &'static Mutex<RefCell<Scheduler>>) {
    let is_preemptive =
        critical_section(|cs_token| {
            spinlock(&TASKMANAGER_LOCK);
            let t = task_manager.borrow(cs_token).borrow().is_preemptive;
            spinunlock(&TASKMANAGER_LOCK);
            t
        });
    if is_preemptive {
        match is_privileged() {
            true => preempt(),
            false => svc_call(),
        };
    }
}

fn preempt() {
    set_pendsv();
}

/// Returns the TaskId of the currently running task in the kernel.
pub fn get_curr_tid(task_manager: &'static Mutex<RefCell<Scheduler>>) -> TaskId {
    critical_section(|cs_token| task_manager.borrow(cs_token).borrow().curr_tid as TaskId)
}

/// The Kernel blocks the tasks mentioned in `tasks_mask`.
pub fn block_tasks(task_manager: &'static Mutex<RefCell<Scheduler>>, tasks_mask: BooleanVector) {
    #[cfg(feature = "system_logger")]
    {
        if logging::get_block_tasks() {
            logging::report(LogEventType::BlockTasks(tasks_mask));
        }
    }
    critical_section(|cs_token| {
        spinlock(&TASKMANAGER_LOCK);
        task_manager
            .borrow(cs_token)
            .borrow_mut()
            .block_tasks(tasks_mask);
        spinunlock(&TASKMANAGER_LOCK);
    })
}

/// The Kernel unblocks the tasks mentioned in tasks_mask.
pub fn unblock_tasks(task_manager: &'static Mutex<RefCell<Scheduler>>, tasks_mask: BooleanVector) {
    #[cfg(feature = "system_logger")]
    {
        if logging::get_unblock_tasks() {
            logging::report(LogEventType::UnblockTasks(tasks_mask));
        }
    }
    critical_section(|cs_token| {
        spinlock(&TASKMANAGER_LOCK);
        task_manager
            .borrow(cs_token)
            .borrow_mut()
            .unblock_tasks(tasks_mask);
        spinunlock(&TASKMANAGER_LOCK);
    })
}

/// The `task_exit` function is called just after a task finishes execution. It marks the current running task as finished and then schedules the next high priority task.
pub fn task_exit(task_manager: &'static Mutex<RefCell<Scheduler>>) {
    critical_section(|cs_token| {
        spinlock(&TASKMANAGER_LOCK);
        let handler = &mut task_manager.borrow(cs_token).borrow_mut();
        let curr_tid = handler.curr_tid;
        #[cfg(feature = "system_logger")]
        {
            if logging::get_task_exit() {
                logging::report(LogEventType::TaskExit(curr_tid as TaskId));
            }
        }
        handler.active_tasks &= !(1 << curr_tid as u32);
        spinunlock(&TASKMANAGER_LOCK);
    });
    schedule(task_manager)
}
/// The Kernel releases the tasks in the `task_mask`, these tasks transition from the waiting to the ready state.
pub fn release(task_manager: &'static Mutex<RefCell<Scheduler>>, tasks_mask: BooleanVector) {
    #[cfg(feature = "system_logger")]
    {
        if logging::get_release() {
            logging::report(LogEventType::ReleaseTasks(tasks_mask));
        }
    }
    critical_section(|cs_token| {
        task_manager
            .borrow(cs_token)
            .borrow_mut()
            .release(tasks_mask)
    });
    schedule(task_manager);
}

/// Enable preemptive scheduling
pub fn enable_preemption(task_manager: &'static Mutex<RefCell<Scheduler>>) {
    critical_section(|cs_token| {
        let handler = &mut task_manager.borrow(cs_token).borrow_mut();
        handler.preempt_disable_count -= 1;
        if handler.preempt_disable_count == 0 {
            handler.is_preemptive = true;
        }
    })
}

/// Disable preemptive scheduling
pub fn disable_preemption(task_manager: &'static Mutex<RefCell<Scheduler>>) {
    critical_section(|cs_token| {
        let handler = &mut task_manager.borrow(cs_token).borrow_mut();
        handler.preempt_disable_count += 1;
        handler.is_preemptive = false;
    })
}
