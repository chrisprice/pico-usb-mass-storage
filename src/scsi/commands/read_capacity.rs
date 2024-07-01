use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ReadCapacity10Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 2..= 5, bits= 0..=7)]
    pub lba: u32,

    #[overlay(bytes= 9..= 9, nested)]
    pub control: Control,
}
