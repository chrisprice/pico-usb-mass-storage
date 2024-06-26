use crate::storage::Storage;

static FS_DUMP: &[u8; 102400] = include_bytes!("../../dumps/linux_partitioned.dump");

pub fn init(storage: &mut Storage) {
    let bytes = storage.as_bytes_mut();

    bytes[..FS_DUMP.len()].copy_from_slice(FS_DUMP);
}
