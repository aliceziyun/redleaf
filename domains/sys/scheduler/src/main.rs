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

        // loop and get next runnable thread if exist
        for (index, thread_meta) in q.iter_mut().enumerate() {
            if let Some(t) = thread_meta.take() {
                match t.state {
                    ThreadState::Runnable => {
                        return Ok(Some(t));
                    }

                    _ => {
                        continue;
                    }

                }
            } else {
                continue;
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