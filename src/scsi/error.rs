use packing::Error as PackingError;

use super::BlockDeviceError;

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum Error {
    UnhandledOpCode,
    /// The identified opcode requires more data than was sent
    InsufficientDataForCommand,
    PackingError(PackingError),
    BlockDeviceError(BlockDeviceError),
    //BulkOnlyTransportError(BulkOnlyTransportError),
}

impl From<PackingError> for Error {
    fn from(e: PackingError) -> Error {
        Error::PackingError(e)
    }
}

impl From<BlockDeviceError> for Error {
    fn from(e: BlockDeviceError) -> Error {
        Error::BlockDeviceError(e)
    }
}
