#![no_std]
#![no_main]

extern crate alloc;
extern crate malloc;

use spin::Mutex;
use syscalls::{Heap, Syscall};

use console::println;

use alloc::{boxed::Box, rc::Rc};
use core::{borrow::Borrow, panic::PanicInfo};

use interface::rref::RRef;

pub fn main(dom_c: &Box<dyn interface::dom_c::DomC>) {
    println!("[E] Init domain E");

    // assert!(dom_c.one_arg(12321).unwrap() == 12321 + 1);

    let return_val = &**dom_c.rref_as_return_value();
    let mut val = return_val.borrow_mut();
    *val += 10;

    assert!(*val == 20);
    println!("[E] domain E execution finishes!")
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain E panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}