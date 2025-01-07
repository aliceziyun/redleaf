#![no_std]
#![no_main]

extern crate alloc;
extern crate malloc;

use interface::rref::RRef;
use interface::rpc::RpcResult;
use interface::sched::{ThreadMetaQueuesInner, ThreadMeta, ThreadState};
use alloc::{boxed::Box, string::String, sync::Arc};
// use spin::Mutex;
use console::println;
use core::{panic::PanicInfo, cell::{Cell, RefCell}, option::Option};
use core::ops::Deref;

struct Scheduler {
    idle: Cell<u64>,
    // current_meta: Cell<ThreadMeta>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            idle: Cell::new(0),
        }
    }
}

impl interface::sched::Scheduler for Scheduler {
    // fn set_queue(&self, queue: &RRef<ThreadMetaQueuesInner>) -> RpcResult<()> {
    //     self.queue = Some(queue);
    // }

    fn set_idle_thread(&self, idle: u64) -> RpcResult<()> {
        self.idle.set(idle);
        Ok(())
    }

    fn get_idle_thread(&self) -> RpcResult<u64> {
        Ok(self.idle.get())
    }

    // fn put_thread_in_queue(&self, metadata: RRef<ThreadMeta>) -> RpcResult<()> {
    //     Ok(())
    // }

    fn get_next(&self, queue: &RRef<ThreadMetaQueuesInner>) -> RpcResult<Option<ThreadMeta>> {
        let mut q = queue.deref().inner_queue.borrow_mut();

        // find thread with hightest prio
        let highest_priority_thread = q.iter()
        .enumerate()
        .filter_map(|(index, thread_meta)| {
            thread_meta.as_ref().and_then(|t| {
                if let ThreadState::Runnable = t.state {
                    Some((index, t.priority))
                } else {
                    None
                }
            })
        })
        .max_by_key(|&(_, priority)| priority)
        .map(|(index, _)| index);

        // TODO: change the metadata here
        if let Some(index) = highest_priority_index {
            if let Some(t) = q[index].take() {
                // println!("next thread is {} with priority {}", t.id, t.priority);
                return Ok(Some(t));
            }
        }

        Ok(None)
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn syscalls::Syscall + Send + Sync>,
    heap: Box<dyn syscalls::Heap + Send + Sync>,
    ints: Box<dyn syscalls::Interrupt + Send + Sync>,
) -> Box<dyn interface::sched::Scheduler> {
    libsyscalls::syscalls::init(s);

    // [alice] use a fixed number now. System should reserve number for the scheduler domain
    interface::rref::init(heap, 12);

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