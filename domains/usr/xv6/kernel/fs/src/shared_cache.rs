use interface::rref::RRef;
use interface::bdev::BDev;

pub type BufferBlock = [u8; BSIZE];

pub struct BufferCache {
    internal: Mutex<BufferCacheInternal>,
    bdev: Box<dyn BDev>,
}

impl BufferCache {
    pub fn new(bdev: Box<dyn BDev>) -> Self {
        Self {
            internal: Mutex::new(BufferCacheInternal::new()),
            bdev,
        }
    }

    pub fn read(&'static self, device: u32, block_number: u32, buffer_slot: &RRef<RefCell<BufferBlock>>) -> RpcResult<> {
        // first get the block buffer
        // buffer: Arc<Mutex<BufferBlockWrapper>>, ensure thread safety
        let (valid, index, buffer) = self.internal.lock().get(device, block_number);
        
        // if not valid, read from bdev
        // TODO: copy happesn here, we can also change read from bdev to RefCell
        if !valid {
            let sector = block_number * (BSIZE / SECTOR_SIZE) as u32;
            let mut guard = buffer.lock();
            (*guard).0 = Some(self.bdev.read(sector, guard.take()).unwrap());
        }

        let buffer_slot = &**buffer_slot;
        if let Some(block) = *guard.0 {
            // bad thing happens here, block's ownership...
            *buffer_slot.borrow_mut() = Some(block);
            Ok()
        } else{
            Err()
        }
    }

    
    // ownership tranfer happens here
    pub fn write(&'static self, device: u32, block_number: u32, buffer_slot: RefCell<BufferBlock>)  -> RpcResult<>{
        // very same as read

        // first get the block buffer

        // if the block exist in cache, overrite it


        // if not exist, load data to cache

        {
            // may need to write cache back to disk
        }

        Ok()
    }
}