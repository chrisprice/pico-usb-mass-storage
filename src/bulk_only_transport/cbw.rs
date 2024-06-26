use defmt::Format;

pub const CBW_LEN: usize = 31;
const CBW_SIGNATURE_LE: [u8; 4] = 0x43425355u32.to_le_bytes();
const MIN_CB_LEN: usize = 1;
const MAX_CB_LEN: usize = 16;

#[repr(u8)]
#[derive(Default, Debug, Copy, Clone, Format)]
pub enum DataDirection {
    Out,
    In,
    #[default]
    NotExpected,
}

#[derive(Default, Debug, Copy, Clone, Format)]
pub struct CommandBlockWrapper {
    pub tag: u32,
    pub data_transfer_len: u32,
    pub direction: DataDirection,
    pub lun: u8,
    pub block_len: usize,
    pub block: [u8; 16],
}

#[derive(Debug, Format)]
pub enum Error {
    InvalidSignature,
    InvalidLength,
}

impl CommandBlockWrapper {
    pub fn from_le_bytes(value: &[u8]) -> Result<Self, Error> {
        if !value.starts_with(&CBW_SIGNATURE_LE) {
            return Err(Error::InvalidSignature);
        }

        let block_len = value[10] as usize;

        if !(MIN_CB_LEN..=MAX_CB_LEN).contains(&block_len) {
            return Err(Error::InvalidLength);
        }

        Ok(CommandBlockWrapper {
            tag: u32::from_le_bytes(value[4..8].try_into().unwrap()),
            data_transfer_len: u32::from_le_bytes(value[8..12].try_into().unwrap()),
            direction: if u32::from_le_bytes(value[4..8].try_into().unwrap()) != 0 {
                if (value[12] & (1 << 7)) > 0 {
                    DataDirection::In
                } else {
                    DataDirection::Out
                }
            } else {
                DataDirection::NotExpected
            },
            lun: value[13] & 0b00001111,
            block_len,
            block: value[15..].try_into().unwrap(), // ok, cause we checked a length
        })
    }
}
