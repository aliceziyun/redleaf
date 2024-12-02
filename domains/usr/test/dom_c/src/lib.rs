#![no_std]
#![no_main]
extern crate alloc;
extern crate malloc;

use syscalls::{Continuation, Heap, Syscall};

use alloc::{boxed::Box, string::String};

use console::println;

use core::panic::PanicInfo;

use interface::rref::RRef;

use interface::rpc::RpcResult;

use core::cell::{Ref, RefCell};

use interface::rref::traits::TypeIdentifiable;

struct DomC {
    test_data: RRef<RefCell<(i32, i32)>>,
}

impl DomC {
    fn new() -> Self {
        Self {
            test_data: RRef::new(RefCell::new((0i32, 0i32))),
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
    fn rref_as_arguement(&self, size: &RRef<RefCell<usize>>){
        let rc_size = &**size;
        {
            let mut value = rc_size.borrow_mut();
            println!("[C] change the interior mutable variable");
            println!("[C] current thread id is: {}", libsyscalls::syscalls::sys_current_thread_id());
            *value += 10;
        }
        // Ok(())
    }

    fn rref_as_return_value (&self) -> &RRef<RefCell<(i32, i32)>> {
        &self.test_data
    }

}

pub fn main() -> Box<dyn interface::dom_c::DomC> {
    println!("Init domain C");

    let thread = libsyscalls::syscalls::sys_current_thread();
    
    let cont =  Continuation {
        
    }
    libsyscalls::syscalls::sys_register_cont(cont);

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
