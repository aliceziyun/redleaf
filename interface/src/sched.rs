use crate::rref::RRef;
use crate::rref::traits::TypeIdentifiable;
use crate::rpc::RpcResult;

use core::cell::RefCell;

pub const MAX_PRIO: usize = 15;
pub const MAX_CPUS: usize = 64;
pub const MAX_CONT: usize = 10;

#[derive(Clone, Copy, Debug)]
pub enum ThreadState {
    Running = 0,
    Runnable = 1,
    Paused = 2,
    Waiting = 3,
    Idle = 4,
    Rebalanced = 5,
}

pub type Priority = usize;

pub struct ThreadMeta {
    pub id: u64,
    pub current_domain_id: u64,
    pub state: ThreadState,
    pub priority: Priority,
    pub affinity: u64,
    pub rebalance: bool,

    pub last_queued: u64,
    pub run_delay: u64,
}

// [alice] it might better to try RRefDeque, and we use brute method to do priority scheduling
pub struct ThreadMetaQueues {
    queue: RRef<ThreadMetaQueuesInner>,
}

impl ThreadMetaQueues {
    pub fn new() -> ThreadMetaQueues {
        ThreadMetaQueues {
            queue: RRef::new(ThreadMetaQueuesInner::new()),
        }
    }

    pub fn add_thread(&self, index: u64, meta: ThreadMeta) {
        self.queue.set_thread(index, meta);
        // let mut inner_queue = self.queue.inner_queue.borrow();
        // inner_queue[index as usize] = Some(meta);
    }

    // [alice] this seems unsafe
    pub fn get_thread_ref(&self, index: u64) -> *const ThreadMeta {
        let inner_queue = self.queue.inner_queue.borrow();
        let meta = inner_queue[index as usize].as_ref().unwrap();
        meta as *const ThreadMeta
    }

    pub fn get_thread(&self, index: u64) -> ThreadMeta {
        let mut inner_queue = self.queue.inner_queue.borrow_mut();
        inner_queue[index as usize].take().unwrap()
    }

    pub fn get_queue_ref<'a>(&'a self) -> &'a RRef<ThreadMetaQueuesInner>{
        // add reference count
        self.queue.borrow();
        &(self.queue)
    }
}

pub struct ThreadMetaQueuesInner {
    pub inner_queue: RefCell<[Option<ThreadMeta>; 256]>,
    clock: u64,     // the running queue clock
    run_delay: u64,
}

impl ThreadMetaQueuesInner {
    pub const fn new() -> ThreadMetaQueuesInner {
        ThreadMetaQueuesInner {
            inner_queue: RefCell::new([
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None,
            ]),
            clock: 12,      // set non zero number
            run_delay: 0,
        }
    }

    pub fn set_thread(&self, index: u64, meta: ThreadMeta) {
        let mut q = self.inner_queue.borrow_mut();
        q[index as usize] = Some(meta);
    }

    pub fn get_clock(&self) -> u64 {
        self.clock
    }

    pub fn add_run_delay(&mut self, run_delay: u64) {
        self.run_delay += run_delay;
    }
}

// impl TypeIdentifiable for ThreadMetaQueuesInner {
//     fn type_id() -> u64 {
//         123455
//     }
// }


#[interface]
pub trait Scheduler: Send {
    fn set_idle_thread(&self, idle: u64) -> RpcResult<()>;
    fn get_idle_thread(&self) -> RpcResult<u64>;

    fn get_next(&self, queue: &RRef<ThreadMetaQueuesInner>) -> RpcResult<Option<ThreadMeta>>;
    fn add_thread(&self, queue: &RRef<ThreadMetaQueuesInner>, meta: RefCell<ThreadMeta>);
}