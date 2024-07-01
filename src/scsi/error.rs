use super::BlockDeviceError;

#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Error {
    UnhandledOpCode,
    /// The identified opcode requires more data than was sent
    InsufficientDataForCommand,
    BlockDeviceError(BlockDeviceError),
    //BulkOnlyTransportError(BulkOnlyTransportError),
}

impl From<BlockDeviceError> for Error {
    fn from(e: BlockDeviceError) -> Error {
        Error::BlockDeviceError(e)
    }
}
