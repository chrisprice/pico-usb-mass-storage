use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct SendDiagnosticCommand {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=5..=7)]
    pub self_test_code: u8,

    #[overlay(bytes=1..=1, bits=4..=4)]
    pub page_format: bool,

    #[overlay(bytes=1..=1, bits=2..=2)]
    pub self_test: bool,

    #[overlay(bytes=1..=1, bits=1..=1)]
    pub device_offline: bool,

    #[overlay(bytes=1..=1, bits=0..=0)]
    pub unit_offline: bool,

    #[overlay(bytes=3..=4)]
    pub parameter_list_length: u16,

    #[overlay(bytes=5..=5, nested)]
    pub control: Control,
}
