use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ReportLunsCommand {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=2..=2, bits=0..=7)]
    pub select_report: u8,

    #[overlay(bytes=6..=9)]
    pub allocation_length: u32,

    #[overlay(bytes=11..=11, nested)]
    pub control: Control,
}
