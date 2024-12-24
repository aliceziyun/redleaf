#![no_std]
#![no_main]

extern crate alloc;
extern crate malloc;

use interface::rref::RRef;
use interface::rpc::RpcResult;
use interface::sched::{ThreadMetaQueuesInner, ThreadMeta};
use alloc::{boxed::Box, string::String, sync::Arc};
// use spin::Mutex;
use console::println;
use core::{panic::PanicInfo};

struct Scheduler {
    idle: u64,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            idle: 0,
        }
    }
}

impl interface::sched::Scheduler for Scheduler {
    fn set_thread_queue(&self, queue: &RRef<ThreadMetaQueuesInner>) -> RpcResult<()> {
        Ok(())
    }

    fn set_idle_thread(&self, idle: u64) -> RpcResult<()> {
        Ok(())
    }

    fn get_idle_thread(&self) -> RpcResult<u64> {
        Ok(1)
    }

    fn put_thread_in_queue(&self, metadata: RRef<ThreadMeta>) -> RpcResult<()> {
        Ok(())
    }

    fn get_next(&self) -> RpcResult<u64> {
        Ok(1)
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    ints: Box<dyn syscalls::Interrupt + Send + Sync>,
) -> Box<dyn interface::sched::Scheduler> {
    libsyscalls::syscalls::init(s);
    interface::rref::init(heap, 12);        // [alice] use a magic number

    let ints_clone = ints.int_clone();
    libsyscalls::syscalls::init_interrupts(ints);

    println!("init domain scheduler!");
    Box::new(Scheduler::new())
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}