use crate::rref::RRef;

use crate::rpc::RpcResult;

use alloc::boxed::Box;

use core::cell::RefCell;

#[interface]
pub trait DomC {
    fn no_arg(&self) -> RpcResult<()>;
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RRef<usize>;
    fn init_dom_c(&self, c: Box<dyn DomC>) -> RpcResult<()>;
    fn rref_as_arguement(&self, size: &RRef<RefCell<usize>>);
    fn rref_as_return_value(&self) -> &RRef<RefCell<usize>>;
}
