use packing::Packed;

#[derive(Clone, Copy, Eq, PartialEq, Debug, Packed)]
#[derive(Default)]
pub enum ResponseDataFormat {
    /// A RESPONSE DATA FORMAT field set to 2h indicates that the standard INQUIRY data
    #[default]
    Standard = 0x2,
}


