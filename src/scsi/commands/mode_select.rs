use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeSelectXCommand {
    // TBD
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeSelect6Command {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=4..=4)]
    pub page_format: bool,

    #[overlay(bytes=1..=1, bits=0..=0)]
    pub save_pages: bool,

    #[overlay(bytes=4..=4, bits=0..=7)]
    pub parameter_list_length: u8,

    #[overlay(bytes=5..=5, nested)]
    pub control: Control,
}
impl From<ModeSelect6Command> for ModeSelectXCommand {
    fn from(_m: ModeSelect6Command) -> Self {
        todo!()
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeSelect10Command {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=4..=4)]
    pub page_format: bool,

    #[overlay(bytes=1..=1, bits=0..=0)]
    pub save_pages: bool,

    #[overlay(bytes=7..=8)]
    pub parameter_list_length: u16,

    #[overlay(bytes=9..=9, nested)]
    pub control: Control,
}
impl From<ModeSelect10Command> for ModeSelectXCommand {
    fn from(_m: ModeSelect10Command) -> Self {
        todo!()
    }
}
