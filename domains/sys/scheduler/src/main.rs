#![no_std]
#![no_main]

use interface::rref::RRef;
use interface::rpc::RpcResult;
use alloc::{boxed::Box, string::String};
use console::println;

struct Scheduler {
    idle: u64,
}


impl Scheduler {
    fn new() -> Self {
        Self {
            idle: -1,
        }
    }
}

impl interface::sched::Scheduler for Scheduler {
    fn set_thread_queue(&self, queue: &RRef<ThreadMetaQueuesInner>) -> RpcResult<()> {
        OK(())
    }

    fn set_idle_thread(&self, idle: u64) -> RpcResult<()> {
        OK(())
    }

    fn get_idle_thread(&self) -> RpcResult<u64> {
        OK(1)
    }

    fn put_thread_in_queue(&self, metadata: RRef<ThreadMeta>) -> RpcResult<()> {
        OK(())
    }

    fn get_next(&self) -> RpcResult<u64> {
        OK(1)
    }
}

#[no_mangle]
pub fn trusted_entry() -> Box<dyn interface::sched::Scheduler> {
    println("init domain scheduler!");
    Box::new(Scheduler::new())
}