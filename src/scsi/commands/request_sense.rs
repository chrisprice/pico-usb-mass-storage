use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct RequestSenseCommand {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 1, bits= 0..=0)]
    pub descriptor_format: bool,

    #[overlay(bytes= 4..= 4, bits= 0..=7)]
    pub allocation_length: u8,

    #[overlay(bytes= 5..= 5, nested)]
    pub control: Control,
}
