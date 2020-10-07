#![no_std]
#![no_main]
#![forbid(unsafe_code)]
#![feature(
    box_syntax,
    const_fn,
    const_raw_ptr_to_usize_cast,
    untagged_unions
)]

#[macro_use]
extern crate alloc;
extern crate core;
extern crate malloc;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate byteorder;


use alloc::boxed::Box;
use console::println;
use core::panic::PanicInfo;
use syscalls::{Heap, Syscall};
use rref::RRefVec;
use usr_interface::error::{Result, ErrorKind};
use usr_interface::net::Net;
use usr_interface::usrnet::UsrNet;
use usr_interface::rpc::RpcResult;

struct Rv6Net {}

impl Rv6Net {
    fn new() -> Self {
        Self {}
    }
}

impl UsrNet for Rv6Net {
    fn clone_usrnet(&self) -> RpcResult<Box<dyn UsrNet>> {
        Ok(box Self {})
    }

    fn listen(&self, port: u16) -> RpcResult<Result<usize>> {
        console::println!("listen {}", port);
        Ok(Err(ErrorKind::UnsupportedOperation))
    }
    fn accept(&self, server: usize) -> RpcResult<Result<usize>> {
        console::println!("accept {}", server);
        Ok(Err(ErrorKind::UnsupportedOperation))
    }

    fn read_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        console::println!("read_socket {}", socket);
        Ok(Err(ErrorKind::UnsupportedOperation))
    }

    fn write_socket(&self, socket: usize, buffer: RRefVec<u8>) -> RpcResult<Result<(usize, RRefVec<u8>)>> {
        console::println!("write_socket {}", socket);
        Ok(Err(ErrorKind::UnsupportedOperation))
    }
}

#[no_mangle]
pub fn trusted_entry(
    s: Box<dyn Syscall + Send + Sync>,
    heap: Box<dyn Heap + Send + Sync>,
    net: Box<dyn Net>,
) -> Box<dyn UsrNet> {
    libsyscalls::syscalls::init(s);
    rref::init(heap, libsyscalls::syscalls::sys_get_current_domain_id());

    println!("init xv6 network driver");
    Box::new(Rv6Net::new())
}

// This function is called on panic.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("xv6net panic: {:?}", info);
    libsyscalls::syscalls::sys_backtrace();
    loop {}
}