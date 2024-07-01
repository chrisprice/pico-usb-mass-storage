use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct TestUnitReadyCommand {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 5..= 5, nested)]
    pub control: Control,
}
