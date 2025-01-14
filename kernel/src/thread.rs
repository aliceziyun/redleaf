// AB: for now lets use a global lock, we'll get rid of it later
//pub static CONTEXT_SWITCH_LOCK: AtomicBool = AtomicBool::new(false);

use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::cell::RefCell;
//use alloc::rc::Rc;
use crate::active_cpus;
use crate::arch::memory::{PAddr, BASE_PAGE_SIZE};
use crate::domain::domain::{Domain, KERNEL_DOMAIN};
use crate::halt;
use crate::interrupt::{disable_irq, enable_irq};
use crate::memory::buddy::BUDDY;
use crate::memory::VSPACE;
use crate::memory::{Frame, PhysicalAllocator};
use crate::tls::cpuid;
use alloc::sync::Arc;
use core::alloc::Layout;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, MutexGuard, Once};

use syscalls::Continuation;
use interface::sched::{MAX_PRIO, MAX_CPUS, MAX_CONT, ThreadState};
use interface::sched::{Priority,  ThreadMeta, ThreadMetaQueues};
use interface::sched::Scheduler as SchedulerDom;

use hashbrown::HashMap;

use crate::generated_domain_create;

extern "C" {
    fn switch(prev_ctx: *mut Context, next_ctx: *mut Context);
}

#[repr(C)]
#[no_mangle]
struct ContinuationState {
    cur: *mut Continuation,
    start: *const Continuation,
    end: *const Continuation,
}

static mut CONT_STATE: ContinuationState = ContinuationState {
    cur: 0 as *mut Continuation,
    start: 0 as *const Continuation,
    end: 0 as *const Continuation,
};

/// This should be a cryptographically secure number, for now
/// just sequential ID
static THREAD_ID: AtomicU64 = AtomicU64::new(0);

const NULL_RETURN_MARKER: usize = 0x0000_0000;

#[thread_local]
pub static CURRENT_META: RefCell<Option<ThreadMetaGlobal>> = RefCell::new(None);
type ThreadMetaGlobal = Arc<Mutex<Option<ThreadMeta>>>;

pub type Link = Option<Arc<Mutex<Thread>>>;

// AB: Watch out! if you change format of this line
// you need to update the grep arguments in checkstack.mk
// Right now we have it as:
//    grep "^pub const STACK_SIZE_IN_PAGES"
pub const STACK_SIZE_IN_PAGES: usize = 4096;

#[repr(C)]
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

// Without unsafe impl Send, the compiler will compain
//   > "`*mut u64` cannot be sent between threads safely"
// This is safe for us because all threads/processes are
// in the same address space and the pointer doesn't point to
// tls variables.
// https://internals.rust-lang.org/t/shouldnt-pointers-be-send-sync-or/8818
unsafe impl core::marker::Send for Thread {}

// TODO: [alice] init this later
#[thread_local]
pub static THREAD_META_ARRAY: Once<ThreadMetaQueues> = Once::new();

#[thread_local]
pub static THREAD_MAP: Once<Arc<Mutex<ThreadMap>>> = Once::new();
type ThreadMap = HashMap<u64, Arc<Mutex<Thread>>>;

pub static SCHEDULER: Once<Arc<Mutex<Box<dyn interface::sched::Scheduler>>>> = Once::new();

pub struct Thread {
    metadata: *const ThreadMeta,

    pub id: u64,
    pub current_domain_id: u64,
    pub name: String,
    pub state: ThreadState,
    priority: Priority,
    affinity: u64,
    rebalance: bool,

    context: Context,
    stack: *mut u64,
    domain: Option<Arc<Mutex<Domain>>>,
    // Next thread in the scheduling queue
    next: Link,
    // Next thread on the domain list
    pub next_domain: Option<Arc<Mutex<Thread>>>,
    // Next thread on the interrupt wait queue list
    pub next_iwq: Option<Arc<Mutex<Thread>>>,

    /// A stack of continuations
    continuations: [Continuation; MAX_CONT],

    // HACK
    continuation_ptr: *mut Continuation,
}

impl Context {
    pub fn new() -> Context {
        Context {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            r11: 0,
            rbx: 0,
            rbp: 0,
            rsp: 0,
            rflags: 0,
        }
    }
}

pub unsafe fn alloc_stack() -> *mut u8 {
    let layout =
        Layout::from_size_align(STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE, BASE_PAGE_SIZE).unwrap();

    let mut frame: Frame = Frame::new(PAddr::from(0), 0);

    if let Some(ref mut fmanager) = *BUDDY.lock() {
        unsafe { frame = fmanager.allocate(layout).unwrap() };
    };

    {
        let ref mut vspace = *VSPACE.lock();
        vspace.set_guard_page(frame.kernel_vaddr());
    }

    let stack_u8 = frame.kernel_vaddr().as_mut_ptr::<u8>();
    stack_u8
}

impl Thread {
    fn init_stack(&mut self, func: extern "C" fn()) {
        /* AB: XXX: die() takes one argument lets pass it via r15 and hope
         * it will stay there */
        self.context.r15 = func as usize;

        let stack_u8 = unsafe { alloc_stack() };

        println!(
            "creating thread {} with stack: {:x}--{:x}",
            self.name,
            stack_u8 as u64,
            stack_u8 as u64 + (crate::thread::STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE) as u64
        );

        /* push 0x0 as the return address for die() so the backtrace
         * terminates correctly */
        unsafe {
            let null_return = stack_u8.offset(
                (STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE - core::mem::size_of::<*const usize>())
                    as isize,
            ) as *mut usize;
            *null_return = NULL_RETURN_MARKER;
        };

        /* push die() on the stack where the switch will pick
         * it up with the ret instruction */
        let die_return = unsafe {
            stack_u8.offset(
                (STACK_SIZE_IN_PAGES * BASE_PAGE_SIZE - 2 * core::mem::size_of::<*const usize>())
                    as isize,
            ) as *mut usize
        };

        unsafe {
            *die_return = die as usize;
        }
        self.stack = stack_u8 as *mut u64;

        /* set the stack pointer to point to die() */
        self.context.rsp = die_return as usize;
    }

    pub fn new(name: &str, func: extern "C" fn()) -> Thread {
        let id = THREAD_ID.fetch_add(1, Ordering::SeqCst);
        let t_meta = ThreadMeta {
            id: id.clone(),
            current_domain_id: 0,
            state: ThreadState::Runnable,
            priority: 0,
            affinity: 0,
            rebalance: false,

            last_queued: 11,    // represent current time
            run_delay: 0,
        };

        let mut array = THREAD_META_ARRAY.r#try().unwrap();
        array.add_thread(id.clone(), t_meta);

        let mut t = Thread {
            name: name.to_string(),

            metadata: array.get_thread_ref(id.clone()),

            id: id.clone(),

            current_domain_id: 0,
            state: ThreadState::Runnable,
            priority: 0,
            affinity: 0,
            rebalance: false,
            
            context: Context::new(),
            stack: 0 as *mut _,
            domain: None,
            next: None,
            next_domain: None,
            next_iwq: None,
            continuations: [Continuation::zeroed(); MAX_CONT],

            // We will update this when we switch to it the first time
            continuation_ptr: 0 as *mut _,
        };

        t.init_stack(func);

        t
    }
}

/// Just make sure die follows C calling convention
/// We don't really need it now as we pass the function pointer via r15
#[no_mangle]
extern "C" fn die(/*func: extern fn()*/) {
    let func: extern "C" fn();

    /* AB: XXX: We assume that the funciton pointer is still in r15 */
    unsafe {
        llvm_asm!("mov $0, r15" : "=r"(func) : : "memory" : "intel", "volatile");
    };

    println!("Starting new thread");

    // Enable interrupts before exiting to user
    enable_irq();
    func();
    disable_irq();

    loop {
        do_yield();
    }
}

fn get_thread(id: &u64) -> Option<Arc<Mutex<Thread>>>{
    let map = THREAD_MAP.r#try().unwrap().lock();
    map.get(id).cloned()
}

fn set_current(meta: ThreadMeta) {
    let mut current = CURRENT_META.borrow_mut();

    if let Some(current_meta) = current.as_ref() {
        let mut current = current_meta.lock();
        current.take();
        *current = Some(meta);
    } else {
        *current = Some(Arc::new(Mutex::new(Some(meta))));
    }
}

fn get_current_meta() -> ThreadMeta {
    let mut current = CURRENT_META.borrow_mut();

    let mut current = current.as_ref().unwrap().lock(); 
    current.take().unwrap()
}

pub fn get_current_ref() -> Arc<Mutex<Thread>> {
    get_current_ref_option().unwrap()
}

pub fn get_current_ref_option() -> Option<Arc<Mutex<Thread>>> {
    let current_meta = CURRENT_META.borrow();
    let current = current_meta.as_ref().unwrap().lock();
    let current = current.as_ref().unwrap();
    get_thread(&current.id)
}

/// Return domain of the current thread
pub fn get_domain_of_current() -> Arc<Mutex<Domain>> {
    let rc_t = get_current_ref();
    let arc_d = rc_t.lock().domain.as_ref().unwrap().clone();

    arc_d
}

pub fn get_current_pthread() -> Box<PThread> {
    Box::new(PThread::new(get_current_ref().clone()))
}

// Kicked from the timer IRQ
pub fn schedule() {
    println!("schedule");
    
    // let mut s = SCHED.borrow_mut();

    // // Process rebalance requests
    // if rb_check_signal(cpuid()) {
    //     s.process_rb_queue();
    // }

    // let next_thread = loop {
    //     let next_thread = match s.next() {
    //         Some(t) => t,
    //         None => {
    //             // Check if current is runnable
    //             // [alice] current might not be initialized
    //             let c = match get_current_ref_option() {
    //                 Some(c) => c,
    //                 None => {
    //                     return;
    //                 }
    //             };

    //             let state = c.lock().state;
    //             match state {
    //                 ThreadState::Runnable => {
    //                     // Current is the only runnable thread, no need to
    //                     // context switch
    //                     trace_sched!("[{}] is the only runnable thread", c.lock().name);
    //                     return;
    //                 }

    //                 ThreadState::Idle => {
    //                     // Idle thread is the only runnable thread, no need to
    //                     // context switch
    //                     trace_sched!("[{}] is the only runnable thread", c.lock().name);
    //                     return;
    //                 }
    //                 _ => {
    //                     println!("set idle");
    //                     // Current is not runnable, and it was the only
    //                     // running thread, switch to idle
    //                     break s.get_idle_thread();
    //                 }
    //             }
    //         }
    //     };

    //     // Need to rebalance this thread, send it to another CPU
    //     if next_thread.lock().rebalance {
    //         rebalance_thread(next_thread);
    //         continue;
    //     }

    //     {
    //         let state = next_thread.lock().state;

    //         // The thread is not runnable, put it back into the passive queue
    //         match state {
    //             ThreadState::Waiting => {
    //                 s.put_thread_in_passive(next_thread.clone());
    //                 continue;
    //             }
    //             _ => {}
    //         }
    //     }

    //     break next_thread;
    // };

    // let c = match get_current() {
    //     Some(t) => t,
    //     None => {
    //         return;
    //     }
    // };

    // trace_sched!("switch to {}", next_thread.lock().name);

    // println!("current thread is {}", c.lock().name);
    // println!("next thread is {}", next_thread.lock().name);

    // // Make next thread current
    // set_current(next_thread.clone());

    // let state = c.lock().state;
    // match state {
    //     ThreadState::Idle => {
    //         // We don't put idle thread in the thread queue
    //         s.set_idle_thread(c.clone());
    //     }
    //     _ => {
    //         // put the old thread back in the scheduling queue
    //         s.put_thread_in_passive(c.clone());
    //     }
    // }

    // drop(s);

    // let prev = unsafe { core::mem::transmute::<*mut Thread, &mut Thread>(&mut *c.lock()) };
    // let next =
    //     unsafe { core::mem::transmute::<*mut Thread, &mut Thread>(&mut *next_thread.lock()) };

    // drop(c);
    // drop(next_thread);

    // unsafe {
    //     // Save current
    //     prev.continuation_ptr = CONT_STATE.cur;

    //     if next.continuation_ptr == (0 as *mut _) {
    //         next.continuation_ptr = &next.continuations as *const _ as *mut _;
    //     }

    //     CONT_STATE.cur = next.continuation_ptr;
    //     CONT_STATE.start = &next.continuations as *const _ as *mut _;
    //     CONT_STATE.end = CONT_STATE.start.offset(MAX_CONT as isize);
    // }

    // unsafe {
    //     switch(&mut prev.context, &mut next.context);
    // }

    // 1. Get the reference of scheduler domain
    let s = SCHEDULER.r#try().unwrap().lock();
    let meta_array = THREAD_META_ARRAY.r#try().unwrap();

    // 2. Use `get_next` in scheduler dom to get next runnable thread
    let (next_thread, next_meta) = loop {
        let next_meta = s.get_next(meta_array.get_queue_ref()).unwrap();
        let next_thread = match &next_meta {
            Some(t) => {
                get_thread(&t.id).unwrap()
            },
    
            None => {
                let c = match get_current_ref_option() {
                    Some(c) => c,
                    None => {return;}
                };
                let state = c.lock().state;
    
                match state {
                    ThreadState::Runnable => {
                        // Current is the only runnable thread, no need to
                        // context switch
                        trace_sched!("[{}] is the only runnable thread", c.lock().name);
                        return;
                    }
    
                    ThreadState::Idle => {
                        // Idle thread is the only runnable thread, no need to
                        // context switch
                        trace_sched!("[{}] is the only runnable thread", c.lock().name);
                        return;
                    }
                    _ => {
                        // Current is not runnable, and it was the only
                        // running thread, switch to idle
                        let idle_id = s.get_idle_thread().unwrap();
                        break (get_thread(&idle_id).unwrap(), next_meta);
                    }
                }
            }
        };

        println!("here");

        {
            let state = next_thread.lock().state;

            // The thread is not runnable, skip it
            match state {
                ThreadState::Waiting => {
                    continue;
                }
                _ => {}
            }
        }

        break (next_thread, next_meta);
    };

    let c = match get_current_ref_option() {
        Some(t) => t,
        None => {
            return;
        }
    };

    println!("current thread is {}", c.lock().name);
    println!("next thread is {}", next_thread.lock().name);
    
    // 3. Swap next thread with current thread
    let state = c.lock().state;
    match state {
        ThreadState::Idle => {
            // We don't put idle thread in the thread queue
            // TODO: set idle thread
            s.set_idle_thread(c.lock().id);
        }
        _ => {
            // put the old thread back in the scheduling queue
            s.add_thread(meta_array.get_queue_ref(), RefCell::new(get_current_meta()));
        }
    }

    set_current(next_meta.unwrap());

    // 4. Context switch
    let prev = unsafe { core::mem::transmute::<*mut Thread, &mut Thread>(&mut *c.lock())};
    let next =
        unsafe { core::mem::transmute::<*mut Thread, &mut Thread>(&mut *next_thread.lock())};

    unsafe {
        // Save current
        prev.continuation_ptr = CONT_STATE.cur;

        if next.continuation_ptr == (0 as *mut _) {
            next.continuation_ptr = &next.continuations as *const _ as *mut _;
        }

        CONT_STATE.cur = next.continuation_ptr;
        CONT_STATE.start = &next.continuations as *const _ as *mut _;
        CONT_STATE.end = CONT_STATE.start.offset(MAX_CONT as isize);
    }

    unsafe {
        switch(&mut prev.context, &mut next.context);
    }
}

// yield is a reserved keyword
pub fn do_yield() {
    trace_sched!("Yield");
    schedule();
}

pub extern "C" fn idle() {
    halt();
}

pub fn create_thread(name: &str, func: extern "C" fn()) -> Box<PThread> {
    let t = Arc::new(Mutex::new(Thread::new(name, func)));
    
    // Add thread to map
    let mut map = THREAD_MAP.r#try().unwrap().lock();
    map.insert(t.lock().id.clone(), t.clone());

    Box::new(PThread::new(Arc::clone(&t)))
}

pub struct PThread {
    pub thread: Arc<Mutex<Thread>>,
}

impl PThread {
    pub const fn new(t: Arc<Mutex<Thread>>) -> PThread {
        PThread { thread: t }
    }
}

impl syscalls::Thread for PThread {
    fn get_id(&self) -> u64 {
        disable_irq();
        let tid = { self.thread.lock().id };
        enable_irq();
        tid
    }

    fn set_affinity(&self, affinity: u64) {
        disable_irq();

        if affinity as u32 >= active_cpus() {
            println!(
                "Error: trying to set affinity:{} for {} but only {} cpus are active",
                affinity,
                self.thread.lock().name,
                active_cpus()
            );
            enable_irq();
            return;
        }

        {
            let mut thread = self.thread.lock();

            println!("Setting affinity:{} for {}", affinity, thread.name);
            thread.affinity = affinity;
            thread.rebalance = true;
            thread.state = ThreadState::Rebalanced;
        }

        enable_irq();
    }

    fn set_priority(&self, prio: u64) {
        disable_irq();

        if prio as usize > MAX_PRIO {
            println!(
                "Error: trying to set priority:{} for {} but MAX_PRIO is only {}",
                prio,
                self.thread.lock().name,
                MAX_PRIO
            );
            enable_irq();
            return;
        }

        {
            let mut thread = self.thread.lock();

            println!("Setting priority:{} for {}", prio, thread.name);
            thread.priority = prio as usize;
        }
        enable_irq();
    }

    fn set_state(&self, state: syscalls::ThreadState) {
        disable_irq();

        {
            let mut thread = self.thread.lock();

            println!("Setting state:{:#?} for {}", state, thread.name);
            match state {
                syscalls::ThreadState::Waiting => {
                    thread.state = ThreadState::Waiting;
                }

                syscalls::ThreadState::Runnable => {
                    thread.state = ThreadState::Runnable;
                }
                _ => {
                    println!("Can't set {:#?} state for {}", state, thread.name);
                }
            }
            drop(thread);
        }
        enable_irq();
    }

    // Drop the guard and goes to sleep atomically
    fn sleep(&self, guard: MutexGuard<()>) {
        disable_irq();

        {
            let mut thread = self.thread.lock();
            thread.state = ThreadState::Waiting;
            drop(guard);
            drop(thread);
        }

        do_yield();

        enable_irq();
    }
}

extern "C" fn kernel_thread_init() {
    // disable_irq();
    // let sched_domain: Box<dyn SchedulerDom> = generated_domain_create::create_domain_scheduler();
    // enable_irq(); 
}

fn init_thread_list() {
    THREAD_META_ARRAY.call_once(|| ThreadMetaQueues::new());
    THREAD_MAP.call_once(|| Arc::new(Mutex::new(HashMap::new())));
}

pub fn init_threads() {
    init_thread_list();

    let (dom, sched) = generated_domain_create::create_domain_scheduler();
    SCHEDULER.call_once(|| Arc::new(Mutex::new(sched)));

    let mut idle = Thread::new("idle", idle);
    // idle.domain = Some(dom.clone());
    // t.current_domain_id = dom.domain.lock().id;
    idle.state = ThreadState::Idle;

    idle.continuation_ptr = &idle.continuations as *const _ as *mut _;

    let s = SCHEDULER.r#try().unwrap().lock();
    s.set_idle_thread(idle.id.clone());

    unsafe {
        asm!("wrgsbase {}", in(reg) (&mut CONT_STATE as *mut ContinuationState));
    }

    // Make idle the current thread
    let idle_meta = THREAD_META_ARRAY.r#try().unwrap().get_thread(idle.id.clone());
    set_current(idle_meta);

    // Add idle to thread map
    let mut map = THREAD_MAP.r#try().unwrap().lock();
    map.insert(idle.id.clone(), Arc::new(Mutex::new(idle)));
}

/* ----------------------- unused ------------------------*/
//#[thread_local]
//static IDLE: RefCell<Option<Arc<Mutex<Thread>>>> = RefCell::new(None);

static mut REBALANCE_FLAGS: RebalanceFlags = RebalanceFlags::new();
static REBALANCE_QUEUES: Mutex<RebalanceQueues> = Mutex::new(RebalanceQueues::new());

#[repr(align(64))]
struct RebalanceFlag {
    rebalance: bool,
}

impl RebalanceFlag {
    const fn new() -> RebalanceFlag {
        RebalanceFlag { rebalance: false }
    }
}

struct RebalanceFlags {
    flags: [RebalanceFlag; MAX_CPUS],
}

// AB: I need this nested data structure hoping that
// it will ensure cache-line alignment
impl RebalanceFlags {
    const fn new() -> RebalanceFlags {
        RebalanceFlags {
            flags: [
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
                RebalanceFlag::new(),
            ],
        }
    }
}

struct RebalanceQueues {
    queues: [Link; MAX_CPUS],
}

unsafe impl Sync for RebalanceQueues {}
unsafe impl Send for RebalanceQueues {}

impl RebalanceQueues {
    const fn new() -> RebalanceQueues {
        RebalanceQueues {
            queues: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None, None, None, None, None, None, None,
            ],
        }
    }
}

fn rb_push_thread(queue: usize, thread: Arc<Mutex<Thread>>) {
    let mut rb_lock = REBALANCE_QUEUES.lock();

    let previous_head = rb_lock.queues[queue].take();

    if let Some(node) = previous_head {
        thread.lock().next = Some(node);
    } else {
        thread.lock().next = None;
    }
    rb_lock.queues[queue] = Some(thread);
}

fn rb_pop_thread(queue: usize) -> Option<Arc<Mutex<Thread>>> {
    let mut rb_lock = REBALANCE_QUEUES.lock();
    let previous_head = rb_lock.queues[queue].take();

    if let Some(node) = previous_head {
        rb_lock.queues[queue] = node.lock().next.take();
        return Some(node);
    } else {
        return None;
    }
}

fn rb_queue_signal(queue: usize) {
    println!("rb queue signal, queue:{}", queue);
    unsafe {
        REBALANCE_FLAGS.flags[queue].rebalance = true;
    };
}

fn rb_queue_clear_signal(queue: usize) {
    println!("rb clear signal, queue:{}", queue);
    unsafe {
        REBALANCE_FLAGS.flags[queue].rebalance = false;
    };
}

fn rb_check_signal(queue: usize) -> bool {
    false
    // unsafe { REBALANCE_FLAGS.flags[queue].rebalance }
}

/// Move thread to another CPU, affinity is CPU number for now
// We push thread on the rebalance queue (at the moment it's not
// on the scheduling queue of this CPU), and signal rebalance request
// for the target CPU
fn rebalance_thread(t: Arc<Mutex<Thread>>) {
    // AB: TODO: treat affinity in a standard way as a bitmask
    // not as CPU number, yes I'm vomiting too
    let cpu_id = t.lock().affinity;

    rb_push_thread(cpu_id as usize, t);
    rb_queue_signal(cpu_id as usize);
}

/// Pop and discard the top continuation
///
/// Assumes IRQs are already turned off.
/// Panics if there is no continuation on the
/// stack.
pub unsafe fn pop_continuation() -> &'static Continuation {
    if (CONT_STATE.cur as *const _) <= CONT_STATE.start {
        panic!("Tried to pop on an empty continuation stack");
    }

    let ptr = CONT_STATE.cur;
    CONT_STATE.cur = CONT_STATE.cur.offset(-1);

    &(*ptr)
}

/// Push a new Continuation to the stack
///
/// Returns a mutable reference that is
/// technically valid for the lifetime of
/// the thread.
///
/// Panics if the continuation stack is full.
pub unsafe fn push_continuation(cont: &Continuation) {
    if (CONT_STATE.cur as *const _) >= CONT_STATE.end {
        panic!("Tried to push to a full continuation stack");
    }

    let mut dst = *(CONT_STATE.cur);

    dst.func = cont.func;
    dst.rflags = cont.rflags;
    dst.r15 = cont.r15;
    dst.r14 = cont.r14;
    dst.r13 = cont.r13;
    dst.r12 = cont.r12;
    dst.r11 = cont.r11;
    dst.rbx = cont.rbx;
    dst.rbp = cont.rbp;
    dst.rsp = cont.rsp;
    dst.rax = cont.rax;
    dst.rcx = cont.rcx;
    dst.rdx = cont.rdx;
    dst.rsi = cont.rsi;
    dst.rdi = cont.rdi;
    dst.r8 = cont.r8;
    dst.r9 = cont.r9;
    dst.r10 = cont.r10;

    CONT_STATE.cur = CONT_STATE.cur.offset(1);
}