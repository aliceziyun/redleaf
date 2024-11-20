#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use spin::Mutex;
use syscalls::{Heap, Syscall};

use alloc::{boxed::Box, rc::Rc};

use console::println;

use core::panic::PanicInfo;

use interface::rref::RRef;

use core::cell::RefCell;

pub fn main(dom_c: &Box<dyn interface::dom_c::DomC>) {
    println!("[D] Init domain D");

    assert!(dom_c.one_arg(12321).unwrap() == 12321 + 1);

    println!("[D] start interior mutability test");

    // let test_im = RRef::new(RefCell::new(0usize));
    // dom_c.rref_as_arguement(&test_im);
    // let im = &*test_im;
    // let value = im.borrow();
    // println!("[D] RefCell value: {}", value);

    let return_val = &**dom_c.rref_as_return_value();
    
    // do modification here
    let mut val = return_val.borrow_mut();
    *val += 10;

    println!("[D] now the value is: {}", val);
    println!("[D] domain D execution finishes!")
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain D panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
