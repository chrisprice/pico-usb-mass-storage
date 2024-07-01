use overlay_macro::overlay;

use crate::scsi::{
    commands::{CommandLength, Control},
    enums::PageControl,
};

/* After a logical unit reset, the device server shall respond in the following manner:
a) if default values are requested, report the default values;
b) if saved values are requested, report valid restored mode parameters, or restore the mode parameters and
report them. If the saved values of the mode parameters are not able to be accessed from the nonvolatile
vendor specific location, the command shall be terminated with CHECK CONDITION status, with the
sense key set to NOT READY. If saved parameters are not implemented, respond as defined in 6.11.5; or
c) if current values are requested and the current values have been sent by the application client via a MODE
SELECT command, the current values shall be returned. If the current values have not been sent, the
device server shall return:
A) the saved values, if saving is implemented and saved values are available; or
B) the default values.
*/

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeSenseXCommand {
    pub command_length: CommandLength,
    pub page_control: PageControl,
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeSense6Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 1, bits= 3..=3)]
    pub disable_block_descriptors: bool,

    #[overlay(bytes= 2..= 2, bits= 6..=7)]
    pub page_control: PageControl,

    #[overlay(bytes= 2..= 2, bits= 0..=5)]
    pub page_code: u8,

    #[overlay(bytes= 3..= 3, bits= 0..=7)]
    pub subpage_code: u8,

    #[overlay(bytes= 4..= 4, bits= 0..=7)]
    pub allocation_length: u8,

    #[overlay(bytes= 5..= 5, nested)]
    pub control: Control,
}
impl From<ModeSense6Command> for ModeSenseXCommand {
    fn from(m: ModeSense6Command) -> Self {
        Self {
            command_length: CommandLength::C6,
            page_control: m.page_control().unwrap(), // FIXME: error handling here and below
        }
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeSense10Command {
    #[overlay(bytes= 0..= 0, bits= 0..=7)]
    pub op_code: u8,

    #[overlay(bytes= 1..= 1, bits= 4..=4)]
    pub long_lba_accepted: bool,

    #[overlay(bytes= 1..= 1, bits= 3..=3)]
    pub disable_block_descriptors: bool,

    #[overlay(bytes= 2..= 2, bits= 6..=7)]
    pub page_control: PageControl,

    #[overlay(bytes= 2..= 2, bits= 0..=5)]
    pub page_code: u8,

    #[overlay(bytes= 3..= 3, bits= 0..=7)]
    pub subpage_code: u8,

    #[overlay(bytes= 8..= 9, bits= 0..=7)]
    pub allocation_length: u16,

    #[overlay(bytes= 10..= 10, nested)]
    pub control: Control,
}
impl From<ModeSense10Command> for ModeSenseXCommand {
    fn from(m: ModeSense10Command) -> Self {
        Self {
            command_length: CommandLength::C10,
            page_control: m.page_control().unwrap(),
        }
    }
}
