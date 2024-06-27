use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(TryFromPrimitive)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum ResponseDataFormat {
    /// A RESPONSE DATA FORMAT field set to 2h indicates that the standard INQUIRY data
    #[default]
    Standard = 0x2,
}
