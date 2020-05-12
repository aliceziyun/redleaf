/// Xv6 system calls

use alloc::boxed::Box;

use crate::vfs::{UsrVFS, NFILE};
use crate::net::Net;
pub use crate::vfs::{FileMode, FileStat};
pub use crate::error::{ErrorKind, Result};

pub trait Xv6: UsrVFS + Net {
    fn clone(&self) -> Xv6Ptr;
    fn sys_spawn_thread(&self, name: &str, func: alloc::boxed::Box<dyn FnOnce() + Send>) -> Box<dyn Thread>;
    fn sys_spawn_domain(&self, path: &str, args: &str, fds: [Option<usize>; NFILE]) -> Result<Box<dyn Thread>>;
    fn sys_rdtsc(&self) -> u64;
}
pub type Xv6Ptr = Box<dyn Xv6 + Send + Sync>;

pub trait Device {
    fn read(&self, data: &mut [u8]);
    fn write(&self, data: &[u8]);
}

pub trait Thread {
    fn join(&self);
}
