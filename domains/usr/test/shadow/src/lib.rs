#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use syscalls::{Heap, Syscall};

use alloc::boxed::Box;
use alloc::sync::Arc;
use console::println;

use core::panic::PanicInfo;

use interface::rref::RRef;

use interface::rpc::RpcResult;
use spin::Mutex;

use core::cell::RefCell;

struct ShadowDomain {
    dom: Option<Box<dyn syscalls::Domain>>,
}

impl ShadowDomain {
    fn new(
        dom: Box<dyn syscalls::Domain>,
    ) -> Self {
        Self {
            dom: Some(dom),
        }
    }
}

struct Shadow {
    dom: Mutex<ShadowDomain>,
}

impl Shadow {
    fn new(
        dom: Box<dyn syscalls::Domain>,
    ) -> Self {
        Self {
            dom: Mutex::new(ShadowDomain::new(dom)),
        }
    }
}

pub fn main(
){
    println!("Init shadow domain");
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain shadow panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
