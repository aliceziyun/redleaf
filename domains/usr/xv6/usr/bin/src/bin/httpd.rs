#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(const_fn, const_raw_ptr_to_usize_cast, untagged_unions)]

extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use alloc::string::String;
use core::panic::PanicInfo;

use libsyscalls::syscalls::sys_println;
use syscalls::{Heap, Syscall};
use usr_interfaces::error::Result;
use usr_interfaces::vfs::{DirectoryEntry, DirectoryEntryRef, FileMode, INodeFileType};
use usr_interfaces::rv6::Xv6;
use usr_interfaces::usrnet::UsrNet;
use usrlib::syscalls::sys_sleep;
use usrlib::{eprintln, println};

use alloc::vec;

use core::fmt;
use core::fmt::Write;

extern crate arrayvec;
use arrayvec::{ArrayVec, ArrayString};

#[macro_use]
use core::include_bytes;

use redhttpd::usrnet::Httpd;

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    rv6: Box<dyn Xv6>,
    args: &str,
) {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());
    usrlib::init(rv6.clone().unwrap());
    println!("Starting rv6 httpd with args: {}", args);

    main(rv6).unwrap();
}

fn main(rv6: Box<dyn Xv6>) -> Result<()> {
    let usrnet = rv6.get_usrnet()?;

    let mut httpd = Httpd::new();

    loop {
        UsrNet::poll(&*usrnet, false);
        httpd.handle(&*usrnet);
        UsrNet::poll(&*usrnet, true);
    }

    Ok(())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    eprintln!("httpd panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}
