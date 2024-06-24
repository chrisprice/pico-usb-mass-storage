use packing::Packed;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Packed, Default)]
pub enum ResponseCode {
    #[default]
    FixedSenseData = 0x70,
    DescriptorSenseData = 0x72,
}
