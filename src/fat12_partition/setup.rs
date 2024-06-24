use crate::Block;

static FS_DUMP: &[u8; 102400] = include_bytes!("../../dumps/linux_partitioned.dump");

pub fn init(storage: &mut [Block]) {
    assert!(storage.len() * crate::BLOCK_SIZE >= FS_DUMP.len());

    // FIXME: need to update the slice length
    let raw = storage as *mut _ as *mut [u8];
    let raw = unsafe { &mut *raw };

    raw[..FS_DUMP.len()].copy_from_slice(FS_DUMP);
}
