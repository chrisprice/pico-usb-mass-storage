use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ReadFormatCapacitiesCommand {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=5..=7)]
    pub logical_unit_number: u8,

    #[overlay(bytes=7..=8)]
    pub allocation_length: u16,

    #[overlay(bytes=11..=11, nested)]
    pub control: Control,
}
