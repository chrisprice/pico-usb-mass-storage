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

    #[allow(dead_code)]
    pub fn as_bytes(&self) -> &[u8] {
        todo!()
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        todo!()
    }

    #[allow(dead_code)]
    pub fn as_blocks(&self) -> &[Block; BLOCKS as usize] {
        &self.0
    }

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
