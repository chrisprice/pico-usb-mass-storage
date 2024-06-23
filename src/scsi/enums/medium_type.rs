use packing::Packed;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Packed)]
#[derive(Default)]
pub enum MediumType {
    #[default]
    Sbc = 0x00,
}

