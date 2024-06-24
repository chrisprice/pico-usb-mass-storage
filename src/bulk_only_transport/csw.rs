use defmt::Format;

use super::cbw::CommandBlockWrapper;

const CSW_LEN: usize = 13;
const CSW_SIGNATURE_LE: [u8; 4] = 0x53425355u32.to_le_bytes();

#[repr(u8)]
#[derive(Default, Copy, Clone, Format)]
pub enum CommandStatus {
    #[default]
    Passed = 0x00,
    Failed = 0x01,
    PhaseError = 0x02,
}

pub fn build_csw(cbw: &CommandBlockWrapper, status: CommandStatus) -> [u8; CSW_LEN] {
    let mut csw = [0u8; CSW_LEN];
    csw[..4].copy_from_slice(CSW_SIGNATURE_LE.as_slice());
    csw[4..8].copy_from_slice(cbw.tag.to_le_bytes().as_slice());
    csw[8..12].copy_from_slice(cbw.data_transfer_len.to_le_bytes().as_slice());
    csw[12..].copy_from_slice(&[status as u8]);
    csw
}
