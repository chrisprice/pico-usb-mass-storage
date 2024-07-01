use crate::scsi::commands::Control;
use overlay_macro::overlay;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ReadXCommand {
    pub lba: u32,
    pub transfer_length: u32,
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Read6Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 3, bits= 0..=4)]
    pub lba: u32,

    #[overlay(bytes= 4..= 4, bits= 0..=7)]
    pub transfer_length: u8,

    #[overlay(bytes= 5..= 5, nested)]
    pub control: Control,
}

impl From<Read6Command> for ReadXCommand {
    fn from(r: Read6Command) -> Self {
        Self {
            lba: r.lba(),
            transfer_length: r.transfer_length().into(),
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Read10Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 1, bits= 5..=7)]
    pub rd_protect: u8,

    #[overlay(bytes= 1..= 1, bits= 4..=4)]
    pub dpo: bool,

    #[overlay(bytes= 1..= 1, bits= 3..=3)]
    pub fua: bool,

    #[overlay(bytes= 1..= 1, bits= 1..=1)]
    pub fua_nv: bool,

    #[overlay(bytes= 2..= 5, bits= 0..=7)]
    pub lba: u32,

    #[overlay(bytes= 6..= 6, bits= 0..=4)]
    pub group_number: u8,

    #[overlay(bytes= 7..= 8, bits= 0..=7)]
    pub transfer_length: u16,

    #[overlay(bytes= 9..= 9, nested)]
    pub control: Control,
}

impl From<Read10Command> for ReadXCommand {
    fn from(r: Read10Command) -> Self {
        Self {
            lba: r.lba(),
            transfer_length: r.transfer_length().into(),
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Read12Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 1, bits= 5..=7)]
    pub rd_protect: u8,

    #[overlay(bytes= 1..= 1, bits= 4..=4)]
    pub dpo: bool,

    #[overlay(bytes= 1..= 1, bits= 3..=3)]
    pub fua: bool,

    #[overlay(bytes= 1..= 1, bits= 1..=1)]
    pub fua_nv: bool,

    #[overlay(bytes= 2..= 5, bits= 0..=7)]
    pub lba: u32,

    #[overlay(bytes= 6..= 9, bits= 0..=7)]
    pub transfer_length: u32,

    #[overlay(bytes= 10..= 10, bits= 0..=4)]
    pub group_number: u8,

    #[overlay(bytes= 11..= 11, nested)]
    pub control: Control,
}

impl From<Read12Command> for ReadXCommand {
    fn from(r: Read12Command) -> Self {
        Self {
            lba: r.lba(),
            transfer_length: r.transfer_length(),
        }
    }
}

/*
TODO: tests
#[test]
fn test_read10_parse() {
    let data = [0, 0, 0, 0x1E, 0x80, 0, 0, 0x8, 0, 0, 0, 0, 0, 0, 0];
    let cmd = Read10Command::parse(&data).unwrap();
    assert_eq!(cmd.lba, 0x1E80);
}
*/
