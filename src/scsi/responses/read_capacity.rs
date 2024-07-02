use overlay_macro::overlay;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ReadCapacity10Response {
    #[overlay(bytes=0..=3)]
    pub max_lba: u32,

    #[overlay(bytes=4..=7)]
    pub block_size: u32,
}
