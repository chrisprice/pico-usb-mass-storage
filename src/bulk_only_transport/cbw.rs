use defmt::Format;

pub const CBW_LEN: usize = 31;
const CBW_SIGNATURE_LE: [u8; 4] = 0x43425355u32.to_le_bytes();

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

impl CommandBlockWrapper {
    #[allow(clippy::result_unit_err)]
    pub fn from_le_bytes(mut value: &[u8]) -> Result<Self, ()> {
        // check if CBW is valid. Spec. 6.2.1
        if !value.starts_with(&CBW_SIGNATURE_LE) {
            todo!("proper error handling");
        }

        value = &value[4..]; // parse CBW (skipping signature)

        const MIN_CB_LEN: u8 = 1;
        const MAX_CB_LEN: u8 = 16;

        let block_len = value[10];

        if !(MIN_CB_LEN..=MAX_CB_LEN).contains(&block_len) {
            return Err(());
        }

        Ok(CommandBlockWrapper {
            tag: u32::from_le_bytes(value[..4].try_into().unwrap()),
            data_transfer_len: u32::from_le_bytes(value[4..8].try_into().unwrap()),
            direction: if u32::from_le_bytes(value[4..8].try_into().unwrap()) != 0 {
                if (value[8] & (1 << 7)) > 0 {
                    DataDirection::In
                } else {
                    DataDirection::Out
                }
            } else {
                DataDirection::NotExpected
            },
            lun: value[9] & 0b00001111,
            block_len: block_len as usize,
            block: value[11..].try_into().unwrap(), // ok, cause we checked a length
        })
    }
}
