use packing::Packed;

use crate::bulk_only_transport::CommandBlock;
use crate::scsi::{
    commands::*,
    enums::*,
    Error,
    packing::ParsePackedStruct,
};

/// A fully parsed and validated SCSI command
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Command {
    Inquiry(InquiryCommand),
    TestUnitReady(TestUnitReadyCommand),
    ReadCapacity(ReadCapacity10Command), // FIXME
    ModeSense(ModeSenseXCommand),
    PreventAllowMediumRemoval(PreventAllowMediumRemovalCommand),
    RequestSense(RequestSenseCommand),
    Read(ReadXCommand),
    Write(WriteXCommand),
    Format(FormatCommand),
    SendDiagnostic(SendDiagnosticCommand),
    ReportLuns(ReportLunsCommand),
    ModeSelect(ModeSelectXCommand),
    StartStopUnit(StartStopUnitCommand),
    ReadFormatCapacities(ReadFormatCapacitiesCommand),
    Verify(Verify10Command), // FIXME?
    SynchronizeCache(SynchronizeCache10Command), // FIXME?
}

impl Command {
    pub fn extract_from_cbw(cbw: &CommandBlock) -> Result<Command, Error> {
        let op_code = OpCode::from_primitive(cbw.bytes[0]).map_err(|_| Error::UnhandledOpCode)?;
        match op_code {
            OpCode::Read6 => Ok(Command::Read(checked_extract::<Read6Command>(cbw)?.into())),
            OpCode::Read10 => Ok(Command::Read(checked_extract::<Read10Command>(cbw)?.into())),
            OpCode::Read12 => Ok(Command::Read(checked_extract::<Read12Command>(cbw)?.into())),
            OpCode::ReadCapacity10 => Ok(Command::ReadCapacity(checked_extract(cbw)?)),
            OpCode::ReadFormatCapacities => Ok(Command::ReadFormatCapacities(checked_extract(cbw)?)),
            OpCode::Inquiry => Ok(Command::Inquiry(checked_extract(cbw)?)),
            OpCode::TestUnitReady => Ok(Command::TestUnitReady(checked_extract(cbw)?)),
            OpCode::ModeSense6 => Ok(Command::ModeSense(checked_extract::<ModeSense6Command>(cbw)?.into())),
            OpCode::ModeSense10 => Ok(Command::ModeSense(checked_extract::<ModeSense10Command>(cbw)?.into())),
            OpCode::ModeSelect6 => Ok(Command::ModeSelect(checked_extract::<ModeSelect6Command>(cbw)?.into())),
            OpCode::ModeSelect10 => Ok(Command::ModeSelect(checked_extract::<ModeSelect10Command>(cbw)?.into())),
            OpCode::PreventAllowMediumRemoval => Ok(Command::PreventAllowMediumRemoval(checked_extract(cbw)?)),
            OpCode::RequestSense => Ok(Command::RequestSense(checked_extract(cbw)?)),
            OpCode::Write6 => Ok(Command::Write(checked_extract::<Write6Command>(cbw)?.into())),
            OpCode::Write10 => Ok(Command::Write(checked_extract::<Write10Command>(cbw)?.into())),
            OpCode::Write12 => Ok(Command::Write(checked_extract::<Write12Command>(cbw)?.into())),
            OpCode::Format => Ok(Command::Format(checked_extract(cbw)?)),
            OpCode::SendDiagnostic => Ok(Command::SendDiagnostic(checked_extract(cbw)?)),
            OpCode::ReportLuns => Ok(Command::ReportLuns(checked_extract(cbw)?)),
            OpCode::StartStopUnit => Ok(Command::StartStopUnit(checked_extract(cbw)?)),
            OpCode::Verify10 => Ok(Command::Verify(checked_extract(cbw)?)),
            OpCode::SynchronizeCache10 => Ok(Command::SynchronizeCache(checked_extract(cbw)?)),
            _ => Err(Error::UnhandledOpCode),
        }
    }
}


fn checked_extract<T>(cbw: &CommandBlock) -> Result<T, Error>
where
    T: ParsePackedStruct,
    Error: From<<T as Packed>::Error>,
    packing::Error: From<<T as Packed>::Error>
{
    if cbw.bytes.len() < T::BYTES {
        Err(Error::InsufficientDataForCommand)?;
    }
    Ok(T::parse(&cbw.bytes)?)
}
