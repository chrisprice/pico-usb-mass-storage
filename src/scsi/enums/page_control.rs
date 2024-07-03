use num_enum::TryFromPrimitive;

#[allow(clippy::enum_variant_names)]
#[repr(u8)]
#[derive(TryFromPrimitive, Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum PageControl {
    /// Current values
    #[default]
    CurrentValues = 0b00,
    /// Changeable values
    ChangeableValues = 0b01,
    /// Default values
    DefaultValues = 0b10,
    /// Saved values
    SavedValues = 0b11,
}
