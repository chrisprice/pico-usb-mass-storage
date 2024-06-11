static FS_DUMP: &[u8; 102400] = include_bytes!("../../dumps/linux_partitioned.dump");
pub fn init(storage: &mut [u8], block_size: u32, blocks: u32) {
    assert!(storage.len() >= (blocks * block_size) as usize);
    assert!(storage.len() >= FS_DUMP.len());
    unsafe {
        let dst = storage.as_mut_ptr();
        core::ptr::copy(FS_DUMP.as_ptr(), dst, FS_DUMP.len());
    }
}
