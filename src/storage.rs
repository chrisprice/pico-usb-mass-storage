pub const BLOCK_SIZE: usize = 512;
pub const BLOCKS: u32 = 200;

#[derive(Copy, Clone)]
pub struct Block([u8; BLOCK_SIZE]);

#[repr(transparent)]
pub struct Storage([Block; BLOCKS as usize]);

impl Storage {
    pub const fn new() -> Self {
        Self([Block::new(); BLOCKS as usize])
    }

    fn byte_len(&self) -> usize {
        self.0.len() * BLOCK_SIZE
    }

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        let p = &self.0 as *const _ as *const u8;
        let len = self.byte_len();

        unsafe { core::slice::from_raw_parts(p, len) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let p = &mut self.0 as *mut _ as *mut u8;
        let len = self.byte_len();

        unsafe { core::slice::from_raw_parts_mut(p, len) }
    }

    #[allow(dead_code)]
    pub fn as_blocks(&self) -> &[Block; BLOCKS as usize] {
        &self.0
    }

    #[allow(dead_code)]
    pub fn as_blocks_mut(&mut self) -> &mut [Block; BLOCKS as usize] {
        &mut self.0
    }

    pub fn block(&self, block: u32) -> &Block {
        &self.0[block as usize]
    }

    pub fn block_mut(&mut self, block: u32) -> &mut Block {
        &mut self.0[block as usize]
    }
}

impl Block {
    const fn new() -> Self {
        Self([0; BLOCK_SIZE])
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}
