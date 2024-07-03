use overlay_macro::overlay;

use crate::scsi::enums::{AdditionalSenseCode, ResponseCode, SenseKey};

#[overlay]
#[derive(Clone, Copy)]
pub struct RequestSenseResponse {
    #[overlay(bytes=0..=0, bits=7..=7)]
    pub valid: bool,

    #[overlay(bytes=0..=0, bits=0..=6)]
    pub response_code: ResponseCode,

    #[overlay(bytes=2..=2, bits=7..=7)]
    pub filemark: bool,

    #[overlay(bytes=2..=2, bits=6..=6)]
    pub end_of_medium: bool,

    #[overlay(bytes=2..=2, bits=5..=5)]
    pub incorrect_length_indicator: bool,

    #[overlay(bytes=2..=2, bits=0..=3)]
    pub sense_key: SenseKey,

    #[overlay(bytes=3..=6)]
    pub information: u32,

    #[overlay(bytes=7..=7, bits=0..=7)]
    /// n-7
    pub additional_sense_length: u8,

    #[overlay(bytes=8..=11, bits=0..=7)]
    pub command_specific_information: u32,

    #[overlay(bytes=12..=13, bits=0..=7)]
    pub additional_sense_code: AdditionalSenseCode,

    #[overlay(bytes=14..=14, bits=0..=7)]
    pub field_replaceable_unit_code: u8,

    #[overlay(bytes=15..=15, bits=7..=7)]
    pub sense_key_specific_valid: bool,

    #[overlay(bytes=15..=17)]
    pub sense_key_specific: u32,

    #[overlay(bytes=18..=252)]
    pub additional_sense_data: [u8; 235],
}

/*
information
command_specifc_information
additional_sense_code
additional_sense_code_qualifier
sense_key_specific
*/

impl Default for RequestSenseResponse {
    fn default() -> Self {
        let mut response = Self::new();

        response.set_valid(true);
        response.set_additional_sense_length(Self::BYTE_LEN as u8 - 7);
        response.set_sense_key_specific_valid(true);
        response.set_additional_sense_data(&[0; 235]);

        response.set_response_code(Default::default());
        response.set_filemark(Default::default());
        response.set_end_of_medium(Default::default());
        response.set_incorrect_length_indicator(Default::default());
        response.set_sense_key(Default::default());
        response.set_information(Default::default());
        response.set_command_specific_information(Default::default());
        response.set_additional_sense_code(Default::default());
        response.set_field_replaceable_unit_code(Default::default());
        response.set_sense_key_specific(Default::default());

        response
    }
}

impl RequestSenseResponse {
    #[allow(dead_code)]
    pub fn reset_status(&mut self) {
        *self = Default::default()
    }
}

/*
    if !descriptor_format
        return fixed sense data
    else
        if descriptor sense data supported
            return descriptor sense data
        else
            return CHECK CONDITION with sense:
                key: ILLEGAL REQUEST
                additional code: INVALID FIELD IN CDB

*/
