use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct FormatCommand {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=6..=7)]
    pub format_protection_information: u8,

    #[overlay(bytes=1..=1, bits=5..=5)]
    pub long_list: bool,

    #[overlay(bytes=1..=1, bits=4..=4)]
    pub format_data: bool,

    #[overlay(bytes=1..=1, bits=3..=3)]
    pub complete_list: bool,

    #[overlay(bytes=1..=1, bits=0..=2)]
    pub defect_list_format: u8,

    #[overlay(bytes=5..=5, nested)]
    pub control: Control,
}
