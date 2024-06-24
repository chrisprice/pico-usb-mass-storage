use packing::Packed;

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Packed, Default)]
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
