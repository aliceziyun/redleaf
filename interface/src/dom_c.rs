use crate::rref::RRef;

use crate::rpc::RpcResult;

use alloc::boxed::Box;

use core::cell::RefCell;

pub struct RawPtr {
    pub value: i32,
}

impl RawPtr {
    pub fn get_raw_pointer(&self) -> *const i32 {
        &self.value as *const i32
    }
}

#[interface]
pub trait DomC {
    fn no_arg(&self) -> RpcResult<()>;
    fn one_arg(&self, x: usize) -> RpcResult<usize>;
    fn one_rref(&self, x: RRef<usize>) -> RRef<usize>;
    fn init_dom_c(&self, c: Box<dyn DomC>) -> RpcResult<()>;
    fn rref_as_arguement(&self, ptr: &RRef<RawPtr>);
    fn rref_as_return_value(&self) -> &RRef<RefCell<(i32,i32)>>;
}
