use crate::interrupt::{disable_irq, enable_irq};
use crate::thread::{do_yield, create_thread};
use x86::bits64::paging::{PAddr, VAddr};
use crate::arch::vspace::{VSpace, ResourceType};
use crate::memory::{paddr_to_kernel_vaddr, MEM_PROVIDER};
use x86::bits64::paging::BASE_PAGE_SIZE;
use alloc::boxed::Box; 
use spin::Mutex;
use alloc::sync::Arc; 
use crate::domain::domain::{Domain}; 
use syscalls::{Thread,PciResource, PciBar};
use crate::round_up;
use core::alloc::{GlobalAlloc, Layout};
use crate::heap::alloc::{alloc_heap, dealloc_heap};

//use crate::domain::domain::BOOTING_DOMAIN; 

extern crate syscalls; 


//pub static BOOT_SYSCALL: BootSyscall = BootSyscall {
//    sys_boot_syscall,
//};

//// AB: I was not able to pass Box<dyn Syscall> as an argument 
//// to user_ep() (maybe it's possible, I didn't have time to 
//// figure it out
//pub fn sys_boot_syscall() -> Box<dyn Syscall> {
//    let pdom = BOOTING_DOMAIN.replace(None);
//
//    enable_irq(); 
//    return pdom.unwrap();
//}

pub struct PDomain {
    domain: Arc<Mutex<Domain>>
}

impl PDomain {
    pub const fn new(dom: Arc<Mutex<Domain>>) -> PDomain {
        PDomain {
            domain: dom,
        }
    }
    
    fn create_domain_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>  {

        println!("sys_create_thread"); 
        let pt = create_thread(name, func);

        let t = pt.thread.clone(); 
    
        let mut d = self.domain.lock();
        d.add_thread(t); 

        println!("Created thread {} for domain {}", pt.thread.lock().name, d.name); 
        pt   
    }
}

impl syscalls::Domain for PDomain { }

impl syscalls::Syscall for PDomain {

    // Print a string 
    fn sys_print(&self, s: &str) {
        disable_irq();
        print!("{}", s);
        enable_irq(); 
    }
    
    // Print a string and a newline
    fn sys_println(&self, s: &str) {
        disable_irq();
        println!("{}", s);
        enable_irq(); 
    }

    fn sys_alloc(&self) -> *mut u8 {
        disable_irq();
        let paddr: PAddr = VSpace::allocate_one_page();
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        println!("sys_alloc: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    fn sys_alloc_huge(&self, sz: u64) -> *mut u8 {
        let how_many = round_up!(sz as usize, BASE_PAGE_SIZE as usize) / BASE_PAGE_SIZE;
        disable_irq();
        let paddr: PAddr = VSpace::allocate_pages(how_many, ResourceType::Memory);
        let vaddr: VAddr = paddr_to_kernel_vaddr(paddr);
        println!("sys_alloc_huge: returning {:x}", vaddr.as_u64());
        enable_irq();
        vaddr.as_mut_ptr()
    }

    // todo: implement free!
    fn sys_free(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // todo: implement free!
    fn sys_free_huge(&self, _p: *mut u8) {
        disable_irq();
        enable_irq();
    }

    // Yield to any thread
    fn sys_yield(&self) {

        disable_irq();
        trace_sched!("sys_yield"); 
        do_yield();
        enable_irq(); 
    }

    // Create a new thread
    fn sys_create_thread(&self, name: &str, func: extern fn()) -> Box<dyn Thread>  {
        disable_irq();
        let pt = self.create_domain_thread(name, func); 
        enable_irq();
        pt
    }

    fn sys_current_thread(&self) -> Box<dyn Thread> {
        disable_irq();
        let current = crate::thread::get_current_pthread();
        enable_irq();
        current
    }

    fn sys_backtrace(&self) {
        use crate::panic::backtrace;
        disable_irq();
        backtrace();
        enable_irq();
    }

    fn sys_alloc_heap(&self, layout: Layout) -> *mut u8 {
        disable_irq();
        let domain_id = self.domain.lock().id;
        let ptr = alloc_heap(domain_id, layout);
        enable_irq();
        ptr
    }

    fn sys_dealloc_heap(&self, ptr: *mut u8, layout: Layout) {
        disable_irq();
        let domain_id = self.domain.lock().id;
        dealloc_heap(domain_id, ptr, layout);
        enable_irq();
    }
}

impl syscalls::CreatePCI for PDomain {
    fn create_domain_pci(&self, pci_resource: Box<dyn syscalls::PciResource>,
                         pci_bar: Box<dyn syscalls::PciBar>)
                    -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::PCI>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_pci(pci_resource, pci_bar);
        enable_irq();
        r
    }

    fn get_pci_resource(&self) -> Box<dyn PciResource> {
        use crate::dev::pci_resource::PCI_RESOURCE;
        disable_irq();
        let pci_r = Box::new(PCI_RESOURCE);
        enable_irq();
        pci_r
    }

    fn get_pci_bar(&self) -> Box<dyn PciBar> {
        use crate::dev::pci_resource::PciDevice;
        disable_irq();
        let pci_dev = Box::new(PciDevice::new());
        enable_irq();
        pci_dev
    }
}

impl syscalls::CreateAHCI for PDomain {
    fn create_domain_ahci(&self, pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::BDev>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_ahci(pci);
        enable_irq();
        r
    }
}

impl syscalls::CreateIxgbe for PDomain {
    fn create_domain_ixgbe(&self, pci: Box<dyn syscalls::PCI>) -> (Box<dyn syscalls::Domain>, Box<dyn syscalls::Net>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_ixgbe(pci);
        enable_irq();
        r
    }
}

impl syscalls::CreateXv6 for PDomain {
    fn create_domain_xv6kernel(&self,
                                ints: Box<dyn syscalls::Interrupt>,
                                create_xv6fs: Box<dyn syscalls::CreateXv6FS>,
                                create_xv6usr: Box<dyn syscalls::CreateXv6Usr>,
                                bdev: Box<dyn syscalls::BDev>) -> Box<dyn syscalls::Domain> {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_xv6kernel(ints, 
                        create_xv6fs, 
                        create_xv6usr, 
                        bdev);
        enable_irq();
        r
    }
}   

impl syscalls::CreateXv6FS for PDomain {
    fn create_domain_xv6fs(&self, bdev: Box<dyn syscalls::BDev>) ->(Box<dyn syscalls::Domain>, Box<dyn syscalls::VFS>) {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_xv6fs(bdev);
        enable_irq();
        r
    }
}   

impl syscalls::CreateXv6Usr for PDomain {
    fn create_domain_xv6usr(&self, name: &str, xv6: Box<dyn syscalls::Xv6>) -> Box<dyn syscalls::Domain> 
    {
        disable_irq();
        let r = crate::domain::create_domain::create_domain_xv6usr(name, xv6);
        enable_irq();
        r
    }
}

#[derive(Clone)]
pub struct Interrupt {
}

impl Interrupt {
    pub const fn new() -> Interrupt {
        Interrupt {
        }
    }
}
 
impl syscalls::Interrupt for Interrupt {

    // Recieve an interrupt
    fn sys_recv_int(&self, int: u8) {
        disable_irq();
        if int as usize > crate::waitqueue::MAX_INT {
            println!("Interrupt {} doesn't exist", int); 
            enable_irq(); 
            return;
        }

        // take the thread off the scheduling queue
        // AB: XXX: for now just mark it as WAITING later we'll 
        // implement a real doubly-linked list and take it out
        let t = crate::thread::get_current_ref(); 
        t.lock().state = crate::thread::ThreadState::Waiting;

        crate::waitqueue::add_interrupt_thread(int as usize, t);
        
        do_yield();
        enable_irq();
    }

    fn int_clone(&self) -> Box<dyn syscalls::Interrupt> {
        Box::new((*self).clone())
    }


}

