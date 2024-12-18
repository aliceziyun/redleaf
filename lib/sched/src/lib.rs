#![no_std]

use core::option::Option;
use core::cell::RefCell;
use interface::rref::traits::TypeIdentifiable;
use interface::rref::RRef;

pub const MAX_PRIO: usize = 15;
pub const MAX_CPUS: usize = 64;
pub const MAX_CONT: usize = 10;

pub type Priority = usize;

pub struct ThreadMeta {
    pub id: u64,
    pub current_domain_id: u64,
    pub state: ThreadState,
    pub priority: Priority,
    pub affinity: u64,
    pub rebalance: bool,
}

// [alice] it might better to try RRefDeque
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
        let mut inner_queue = self.queue.innerQueue.borrow_mut();
        inner_queue[index as usize] = Some(meta);
    }

    // [alice] this seems unsafe
    pub fn get_thread_ref(&self, index: u64) -> *const ThreadMeta {
        let inner_queue = self.queue.innerQueue.borrow();
        let meta = inner_queue[index as usize].as_ref().unwrap();
        meta as *const ThreadMeta
    }
}

struct ThreadMetaQueuesInner {
    innerQueue: RefCell<[Option<ThreadMeta>; 256]>
}

impl ThreadMetaQueuesInner {
    const fn new() -> ThreadMetaQueuesInner {
        ThreadMetaQueuesInner {
            innerQueue: RefCell::new([
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
        }
    }
}

impl TypeIdentifiable for ThreadMetaQueuesInner {
    fn type_id() -> u64 {
        11514561
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThreadState {
    Running = 0,
    Runnable = 1,
    Paused = 2,
    Waiting = 3,
    Idle = 4,
    Rebalanced = 5,
}
