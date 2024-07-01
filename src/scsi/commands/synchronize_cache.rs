use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct SynchronizeCache10Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 1, bits= 1..=1)]
    pub immediate: bool,

    #[overlay(bytes= 2..= 5, bits= 0..=7)]
    pub lba: u32,

    #[overlay(bytes= 6..= 6, bits= 0..=4)]
    pub group_number: u8,

    #[overlay(bytes= 7..= 8, bits= 0..=7)]
    pub number_of_blocks: u16,

    #[overlay(bytes= 9..= 9, nested)]
    pub control: Control,
}
