use overlay_macro::overlay;

use crate::scsi::commands::Control;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct StartStopUnitCommand {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub op_code: u8,

    #[overlay(bytes=1..=1, bits=0..=0)]
    pub immediate: bool,

    #[overlay(bytes=3..=3, bits=0..=3)]
    pub power_condition_modifier: u8,

    #[overlay(bytes=4..=4, bits=4..=7)]
    pub power_condition: u8,

    #[overlay(bytes=4..=4, bits=2..=2)]
    pub no_flush: bool,

    #[overlay(bytes=4..=4, bits=1..=1)]
    pub load_eject: bool,

    #[overlay(bytes=4..=4, bits=0..=0)]
    pub start: bool,

    #[overlay(bytes=5..=5, nested)]
    pub control: Control,
}
