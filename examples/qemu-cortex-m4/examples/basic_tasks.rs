#![no_std]
#![no_main]

extern crate panic_halt;
extern crate stm32f4;

use cortex_m_rt::entry;
use cortex_m_semihosting::hprintln;
use cortex_m::interrupt::Mutex;

use hartex_rust::process::*;
use hartex_rust::resource::init_peripherals;
use hartex_rust::types::*;
use hartex_rust::{init, spawn};
use hartex_rust::helper::generate_task_mask;

#[entry]
fn main() -> ! {
    let peripherals = init_peripherals().unwrap();

    spawn!(thread1, 1, {
        hprintln!("task 1");
    });
    spawn!(thread2, 2, {
        hprintln!("task 2");
    });
    spawn!(thread3, 3, {
        hprintln!("task 3");
    });

    init!(true);
    release(generate_task_mask(&[1,2,3]));
    start_kernel(&mut peripherals.access().unwrap().borrow_mut(), 150_000);
    
    loop {}
}
