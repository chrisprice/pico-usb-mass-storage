use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct WriteXCommand {
    pub lba: u32,
    pub transfer_length: u32,
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Write6Command {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=3, bits=4..24)]
    pub lba: u32,

    #[overlay(bytes=4..=4, bits=0..=7)]
    pub transfer_length: u8,

    #[overlay(bytes=5..=5, nested)]
    pub control: Control,
}
impl From<Write6Command> for WriteXCommand {
    fn from(w: Write6Command) -> Self {
        Self {
            lba: w.lba(),
            transfer_length: w.transfer_length().into(),
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Write10Command {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=5..=7)]
    pub wr_protect: u8,

    #[overlay(bytes=1..=1, bits=4..=4)]
    pub dpo: bool,

    #[overlay(bytes=1..=1, bits=3..=3)]
    pub fua: bool,

    #[overlay(bytes=1..=1, bits=1..=1)]
    pub fua_nv: bool,

    #[overlay(bytes=2..=5)]
    pub lba: u32,

    #[overlay(bytes=6..=6, bits=0..=4)]
    pub group_number: u8,

    #[overlay(bytes=7..=8)]
    pub transfer_length: u16,

    #[overlay(bytes=9..=9, nested)]
    pub control: Control,
}
impl From<Write10Command> for WriteXCommand {
    fn from(w: Write10Command) -> Self {
        Self {
            lba: w.lba(),
            transfer_length: w.transfer_length().into(),
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Write12Command {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=5..=7)]
    pub wr_protect: u8,

    #[overlay(bytes=1..=1, bits=4..=4)]
    pub dpo: bool,

    #[overlay(bytes=1..=1, bits=3..=3)]
    pub fua: bool,

    #[overlay(bytes=1..=1, bits=1..=1)]
    pub fua_nv: bool,

    #[overlay(bytes=2..=5)]
    pub lba: u32,

    #[overlay(bytes=6..=9)]
    pub transfer_length: u32,

    #[overlay(bytes=10..=10, bits=0..=4)]
    pub group_number: u8,

    #[overlay(bytes=11..=11, nested)]
    pub control: Control,
}
impl From<Write12Command> for WriteXCommand {
    fn from(w: Write12Command) -> Self {
        Self {
            lba: w.lba(),
            transfer_length: w.transfer_length(),
        }
    }
}
