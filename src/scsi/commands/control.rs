use overlay_macro::overlay;

/// This is the last byte on all commands
#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct Control {
    #[overlay(bytes= 0..= 0, bits= 6..=7)]
    pub vendor_specific: u8,

    #[overlay(bytes= 0..= 0, bits= 2..=2)]
    pub normal_aca: bool,
}
