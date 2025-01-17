#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use syscalls::{Continuation, Heap, Syscall};

use alloc::{boxed::Box, string::String};

use console::println;

use core::{ops::Deref, panic::PanicInfo};

use interface::rref::RRef;

use interface::rpc::RpcResult;

use core::cell::{Ref, RefCell};

use interface::rref::traits::TypeIdentifiable;

use interface::dom_c::RawPtr;

struct DomC {
    test_data: RRef<RefCell<(i32, i32)>>,
    // test2: RRef<ThreadMetaQueuesInner>,
}

impl DomC {
    fn new() -> Self {
        Self {
            test_data: RRef::new(RefCell::new((0i32, 0i32))),
            // test2: RRef::new(ThreadMetaQueuesInner::new()),
        }
    }
}

impl interface::dom_c::DomC for DomC {
    fn no_arg(&self) -> RpcResult<()> {
        Ok(())
    }

    fn one_arg(&self, x: usize) -> RpcResult<usize> {
        #[cfg(feature = "unwind")]
        {
            let start = libtime::get_rdtsc();
            assert!((start & 0x100) != 0x100);
        }
        Ok(x + 1)
    }

    fn one_rref(&self, mut x: RRef<usize>) -> RRef<usize> {
        *x += 1;
        x
    }

    fn init_dom_c(&self, c: Box<dyn interface::dom_c::DomC>) -> RpcResult<()> {
        Ok(())
    }

    // Test RRef with smart pointer
    fn rref_as_arguement(&self, ptr: &RRef<RawPtr>){
        // let bc = size.borrow_count();
        // println!("[C] bc is: {}", bc);
        let ptr = ptr.deref();
        ptr.get_raw_pointer();
    }

    fn rref_as_return_value (&self) -> &RRef<RefCell<(i32, i32)>> {
        &self.test_data
    }
}

pub fn main() -> Box<dyn interface::dom_c::DomC> {
    println!("Init domain C");
    Box::new(DomC::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("domain C panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    libsyscalls::syscalls::sys_test_unwind();
    loop {}
}
