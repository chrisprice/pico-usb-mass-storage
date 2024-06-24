use crate::scsi::{commands::Control, packing::ParsePackedStruct};
use packing::Packed;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Packed)]
#[packed(big_endian, lsb0)]
pub struct TestUnitReadyCommand {
    #[pkd(7, 0, 0, 0)]
    pub op_code: u8,

    #[pkd(7, 0, 5, 5)]
    pub control: Control,
}

impl ParsePackedStruct for TestUnitReadyCommand {}
