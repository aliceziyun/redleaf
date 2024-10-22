use crate::rref::RRef;

use crate::rpc::RpcResult;

use alloc::boxed::Box;

use core::cell::RefCell;
#[interface]
pub trait DomC {
    fn no_arg(&self) -> RpcResult<()>;
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RpcResult<RRef<usize>>;
    fn init_dom_c(&self, c: Box<dyn DomC>) -> RpcResult<()>;
    fn test_rref_with_smart_pointer(&self, size: &RRef<RefCell<usize>>) -> RpcResult<()>;
}
