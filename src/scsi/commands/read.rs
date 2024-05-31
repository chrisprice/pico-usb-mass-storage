use overlay_macro::overlay;
use packing::Packed;
//use crate::scsi::commands::Control;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ReadXCommand {
    pub lba: u32,
    pub transfer_length: u32,
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Read6Command {
    #[bit_byte(7, 0, 0, 0)]
    pub op_code: u8,

    #[bit_byte(4, 0, 1, 3)]
    pub lba: u32,

    #[bit_byte(7, 0, 4, 4)]
    pub transfer_length: u8,

    #[bit_byte(7, 0, 5, 5)]
    pub control: u8, //Control,
}

impl From<Read6Command> for ReadXCommand {
    fn from(r: Read6Command) -> Self {
        Self {
            lba: r.lba().into(),
            transfer_length: r.transfer_length().into(),
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Read10Command {
    #[bit_byte(7, 0, 0, 0)]
    pub op_code: u8,

    #[bit_byte(7, 5, 1, 1)]
    pub rd_protect: u8,

    #[bit_byte(4, 4, 1, 1)]
    pub dpo: bool,

    #[bit_byte(3, 3, 1, 1)]
    pub fua: bool,

    #[bit_byte(1, 1, 1, 1)]
    pub fua_nv: bool,

    #[bit_byte(7, 0, 2, 5)]
    pub lba: u32,

    #[bit_byte(4, 0, 6, 6)]
    pub group_number: u8,

    #[bit_byte(7, 0, 7, 8)]
    pub transfer_length: u16,

    #[bit_byte(7, 0, 9, 9)]
    pub control: u8, //Control,
}

impl From<Read10Command> for ReadXCommand {
    fn from(r: Read10Command) -> Self {
        Self {
            lba: r.lba().into(),
            transfer_length: r.transfer_length().into(),
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Read12Command {
    #[bit_byte(7, 0, 0, 0)]
    pub op_code: u8,

    #[bit_byte(7, 5, 1, 1)]
    pub rd_protect: u8,

    #[bit_byte(4, 4, 1, 1)]
    pub dpo: bool,

    #[bit_byte(3, 3, 1, 1)]
    pub fua: bool,

    #[bit_byte(1, 1, 1, 1)]
    pub fua_nv: bool,

    #[bit_byte(7, 0, 2, 5)]
    pub lba: u32,

    #[bit_byte(7, 0, 6, 9)]
    pub transfer_length: u32,

    #[bit_byte(4, 0, 10, 10)]
    pub group_number: u8,

    #[bit_byte(7, 0, 11, 11)]
    pub control: u8, //Control,
}

impl From<Read12Command> for ReadXCommand {
    fn from(r: Read12Command) -> Self {
        Self {
            lba: r.lba().into(),
            transfer_length: r.transfer_length().into(),
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
