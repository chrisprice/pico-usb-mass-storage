use num_enum::TryFromPrimitive;
use overlay_macro::overlay;

use crate::scsi::enums::MediumType;

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeParameterHeader6 {
    #[overlay(bytes=0..=0, bits=0..=7)]
    pub mode_data_length: u8,

    #[overlay(bytes=1..=1, bits=0..=7)]
    pub medium_type: MediumType,

    #[overlay(bytes=2..=2,  nested)]
    pub device_specific_parameter: SbcDeviceSpecificParameter,

    #[overlay(bytes=3..=3, bits=0..=7)]
    pub block_descriptor_length: u8,
}
impl Default for ModeParameterHeader6 {
    fn default() -> Self {
        let mut header = Self::new();
        header.set_mode_data_length(Self::BYTE_LEN as u8 - 1);
        header.set_medium_type(Default::default());
        *header.device_specific_parameter_mut() = Default::default();
        header.set_block_descriptor_length(0);
        header
    }
}
impl ModeParameterHeader6 {
    /// Increase the relevant length fields to indicate the provided page follows this header
    /// can be called multiple times but be aware of the max length allocated by CBW
    pub fn increase_length_for_page(&mut self, page_code: PageCode) {
        self.set_mode_data_length(
            self.mode_data_length()
                + match page_code {
                    PageCode::CachingModePage => CachingModePage::BYTE_LEN as u8,
                },
        );
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct ModeParameterHeader10 {
    #[overlay(bytes=0..=1)]
    pub mode_data_length: u16,

    #[overlay(bytes=2..=2, bits=0..=7)]
    pub medium_type: MediumType,

    #[overlay(bytes=3..=3, nested)]
    pub device_specific_parameter: SbcDeviceSpecificParameter,

    #[overlay(bytes=4..=4, bits=0..=0)]
    pub long_lba: bool,

    #[overlay(bytes=6..=7)]
    pub block_descriptor_length: u16,
}
impl Default for ModeParameterHeader10 {
    fn default() -> Self {
        let mut header = Self::new();
        header.set_mode_data_length(Self::BYTE_LEN as u16 - 2);
        header.set_medium_type(Default::default());
        *header.device_specific_parameter_mut() = Default::default();
        header.set_long_lba(Default::default());
        header.set_block_descriptor_length(0);
        header
    }
}
impl ModeParameterHeader10 {
    /// Increase the relevant length fields to indicate the provided page follows this header
    /// can be called multiple times but be aware of the max length allocated by CBW
    #[allow(dead_code)]
    pub fn increase_length_for_page(&mut self, page_code: PageCode) {
        self.set_mode_data_length(
            self.mode_data_length()
                + match page_code {
                    PageCode::CachingModePage => CachingModePage::BYTE_LEN as u16,
                },
        );
    }
}

#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub struct SbcDeviceSpecificParameter {
    #[overlay(bytes=0..=0, bits=7..=7)]
    pub write_protect: bool,

    #[overlay(bytes=0..=0, bits=4..=4)]
    pub disable_page_out_and_force_unit_access_available: bool,
}

#[repr(u8)]
#[derive(TryFromPrimitive, Clone, Copy, Eq, PartialEq, Debug)]
pub enum PageCode {
    CachingModePage = 0x08,
}

/// This is only a partial implementation, there are a whole load of extra
/// fields defined in SBC-3 6.4.5
/// Default config is no read or write cache
#[overlay]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct CachingModePage {
    #[overlay(bytes=0..=0, bits=0..=5)]
    pub page_code: PageCode,

    #[overlay(bytes=1..=1, bits=0..=7)]
    pub page_length: u8,

    #[overlay(bytes=2..=2, bits=2..=2)]
    pub write_cache_enabled: bool,

    #[overlay(bytes=2..=2, bits=0..=0)]
    pub read_cache_disable: bool,
}
impl Default for CachingModePage {
    fn default() -> Self {
        let mut mode = Self::new();
        mode.set_page_code(PageCode::CachingModePage);
        mode.set_page_length(Self::BYTE_LEN as u8);
        mode.set_write_cache_enabled(false);
        mode.set_read_cache_disable(true);
        mode
    }
}
