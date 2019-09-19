use crate::common::list::{List, ListLink, ListNode};

//use alloc::boxed::Box;
// AB: for now lets use a global lock, we'll get rid of it later
//pub static CONTEXT_SWITCH_LOCK: AtomicBool = AtomicBool::new(false);

const MAX_PRIO: usize = 10;

pub enum State {
    Running = 0,
    Runnable = 1,
    Paused = 2, 
}

const STACK_SIZE: usize = 4096 * 64;
struct Stack {
    chars: [u8; STACK_SIZE],
}

#[derive(Clone, Debug)]
pub struct Context {
  r15: usize,
  r14: usize,
  r13: usize, 
  r12: usize,
  r11: usize, 
  rbx: usize, 
  rbp: usize,  
  rsp: usize,
  rflags: usize,
}

type Priority = usize;

//type Link = Option<Box<Thread>>;

struct Thread<'a> {
    state: State, 
    priority: Priority, 
    context: Context,
    //stack: * mut Stack,
    // Next thread in the scheduling queue
    next: ListLink<'a, Thread<'a>>,
}


struct SchedulerQueue<'a> {
    highest: Priority,
    //next: List<'a, Thread<'a>>,
    queues: [List<'a, Thread<'a>>; 1 /*MAX_PRIO*/],
}

pub struct Scheduler<'a> {
    active: bool,
    active_queue: SchedulerQueue<'a>,
    passive_queue: SchedulerQueue<'a>,
}

impl <'a> ListNode<'a, Thread<'a>> for Thread<'a> {
    fn next(&'a self) -> &'a ListLink<'a, Thread<'a>> {
        &self.next
    }
}


impl <'a> SchedulerQueue<'a> {

    pub fn new() -> SchedulerQueue<'a> {
        SchedulerQueue {
            highest: 0,
            queues: {[List::new()]},
        }
    }


    // Add thread to the queue that matches thread's priority
    pub fn put_thread(&mut self, thread: &'a mut Thread<'a>) {
        let prio = thread.priority;
    
        self.queues[prio].push_head(thread);

        if self.highest < prio {
            self.highest = prio
        }
    }

    
    // Try to get the thread with the highest priority
    pub fn get_highest(&mut self) -> Option<&mut Thread<'a>> {
        loop {
            match self.queues[self.highest].pop_head() {
                None => {
                    if self.highest == 0 {
                        return None;
                    }
                    self.highest += 1;
                },
                Some(t) => {
                    return Some(t);
                },
            }
        }
    }

}
impl <'a> Scheduler<'a> {

    pub fn new() -> Scheduler<'a> {
        Scheduler {
            active: true,
            active_queue: SchedulerQueue::new(),
            passive_queue: SchedulerQueue::new(),
        }
    }

    pub fn put_in_passive(&mut self, thread: &'a mut Thread<'a>) {
        /* put thread in the currently passive queue */
        if !self.active {
            self.active_queue.put_thread(thread)
        } else {
            self.passive_queue.put_thread(thread)
        }
    }

    pub fn get_next_active(&mut self) -> Option<&mut Thread<'a>> {
        if self.active {
            self.active_queue.get_highest()
        } else {
            self.passive_queue.get_highest()
        }
    }

    
    pub fn get_next(&mut self) -> Option<&mut Thread<'a>> {
        //loop{

            match self.get_next_active() {
                Some(t) => return Some(t),
                None => None,
            }

            //self.flip_queues();
        //}
        // t t1 = self.get_next();
        //        t1
        //    },

        //loop {
        /*
                if let Some(t) = self.get_next_active() {
                    return t; 
                } else { 
                     self.flip_queues();
                     return self.get_next();
                }
        */
    }   

    // Flip active and passive queue making active queue passive
    pub fn flip_queues(&mut self) {
        if self.active {
            self.active = false
        } else {
            self.active = true
        }
    }
    
}

pub fn schedule(s: &mut Scheduler, current_thread: &mut Thread) {

    let next_thread: &mut Thread = loop {
        if let Some(t) = s.get_next() {
            break t;
        }
        s.flip_queues();
    };

    current_thread.context.switch_to(&mut (*next_thread).context);
}

impl Context {
    pub fn new() -> Context {
        Context {
            rflags: 0,
            rbx: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rbp: 0,
            rsp: 0
        }
    }

    /// Switch to the next context by restoring its stack and registers
    #[cold]
    #[inline(never)]
    #[naked]
    pub unsafe fn switch_to(&mut self, next: &mut Context) {
        //asm!("fxsave64 [$0]" : : "r"(self.fx) : "memory" : "intel", "volatile");
        //self.loadable = true;
        //if next.loadable {
        //    asm!("fxrstor64 [$0]" : : "r"(next.fx) : "memory" : "intel", "volatile");
        //}else{
        //    asm!("fninit" : : : "memory" : "intel", "volatile");
        //}

        //asm!("mov $0, cr3" : "=r"(self.cr3) : : "memory" : "intel", "volatile");
        //if next.cr3 != self.cr3 {
        //    asm!("mov cr3, $0" : : "r"(next.cr3) : "memory" : "intel", "volatile");
        //}

        asm!("pushfq ; pop $0" : "=r"(self.rflags) : : "memory" : "intel", "volatile");
        asm!("push $0 ; popfq" : : "r"(next.rflags) : "memory" : "intel", "volatile");

        asm!("mov $0, rbx" : "=r"(self.rbx) : : "memory" : "intel", "volatile");
        asm!("mov rbx, $0" : : "r"(next.rbx) : "memory" : "intel", "volatile");

        asm!("mov $0, r12" : "=r"(self.r12) : : "memory" : "intel", "volatile");
        asm!("mov r12, $0" : : "r"(next.r12) : "memory" : "intel", "volatile");

        asm!("mov $0, r13" : "=r"(self.r13) : : "memory" : "intel", "volatile");
        asm!("mov r13, $0" : : "r"(next.r13) : "memory" : "intel", "volatile");

        asm!("mov $0, r14" : "=r"(self.r14) : : "memory" : "intel", "volatile");
        asm!("mov r14, $0" : : "r"(next.r14) : "memory" : "intel", "volatile");

        asm!("mov $0, r15" : "=r"(self.r15) : : "memory" : "intel", "volatile");
        asm!("mov r15, $0" : : "r"(next.r15) : "memory" : "intel", "volatile");

        asm!("mov $0, rsp" : "=r"(self.rsp) : : "memory" : "intel", "volatile");
        asm!("mov rsp, $0" : : "r"(next.rsp) : "memory" : "intel", "volatile");

        asm!("mov $0, rbp" : "=r"(self.rbp) : : "memory" : "intel", "volatile");
        asm!("mov rbp, $0" : : "r"(next.rbp) : "memory" : "intel", "volatile");
    }

}

/* 
unsafe fn runnable(thread: &Thread) -> bool {
    thread.status == Status::Runnable
}

/// Do not call this while holding locks!
pub unsafe fn switch() -> bool {
    use core::ops::DerefMut;

    // Set the global lock to avoid the unsafe operations below from causing issues
    while arch::CONTEXT_SWITCH_LOCK.compare_and_swap(false, true, Ordering::SeqCst) {
        interrupt::pause();
    }

    let cpu_id = crate::cpu_id();

    let from_ptr;
    let mut to_ptr = 0 as *mut Context;
    {
        let contexts = contexts();
        {
            let context_lock = contexts
                .current()
                .expect("context::switch: not inside of context");
            let mut context = context_lock.write();
            from_ptr = context.deref_mut() as *mut Context;
        }

        for (_pid, context_lock) in contexts.iter() {
            let mut context = context_lock.write();
            update(&mut context, cpu_id);
        }

        for (pid, context_lock) in contexts.iter() {
            if *pid > (*from_ptr).id {
                let mut context = context_lock.write();
                if runnable(&mut context, cpu_id) {
                    to_ptr = context.deref_mut() as *mut Context;
                    break;
                }
            }
        }

        if to_ptr as usize == 0 {
            for (pid, context_lock) in contexts.iter() {
                if *pid < (*from_ptr).id {
                    let mut context = context_lock.write();
                    if runnable(&mut context, cpu_id) {
                        to_ptr = context.deref_mut() as *mut Context;
                        if (&mut *to_ptr).ksig.is_none() {
                            to_sig = context.pending.pop_front();
                        }
                        break;
                    }
                }
            }
        }
    };

    // Switch process states, TSS stack pointer, and store new context ID
    if to_ptr as usize != 0 {
        (&mut *from_ptr).state = Runnable;
        (&mut *to_ptr).state = Running;
        //if let Some(ref stack) = (*to_ptr).kstack {
        //    gdt::set_tss_stack(stack.as_ptr() as usize + stack.len());
        //}
        gdt::set_tcb((&mut *to_ptr).id.into());
        CONTEXT_ID.store((&mut *to_ptr).id, Ordering::SeqCst);
    }

    // Unset global lock before switch, as arch is only usable by the current CPU at this time
    arch::CONTEXT_SWITCH_LOCK.store(false, Ordering::SeqCst);

    if to_ptr as usize == 0 {
        // No target was found, return

        false
    } else {

        (&mut *from_ptr).arch.switch_to(&mut (&mut *to_ptr).arch);
        true
    }
}
*/
