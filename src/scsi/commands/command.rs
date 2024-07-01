use crate::bulk_only_transport::CommandBlock;
use crate::scsi::{commands::*, enums::*, Error};

/// A fully parsed and validated SCSI command
#[derive(Clone, Copy, Eq, PartialEq, Debug, defmt::Format)] // FIXME: don't use Debug2Format
pub enum Command {
    Inquiry(#[defmt(Debug2Format)] InquiryCommand),
    TestUnitReady(#[defmt(Debug2Format)] TestUnitReadyCommand),
    ReadCapacity(#[defmt(Debug2Format)] ReadCapacity10Command), // FIXME: ReadCapacity16
    ModeSense(#[defmt(Debug2Format)] ModeSenseXCommand),
    PreventAllowMediumRemoval(#[defmt(Debug2Format)] PreventAllowMediumRemovalCommand),
    RequestSense(#[defmt(Debug2Format)] RequestSenseCommand),
    Read(#[defmt(Debug2Format)] ReadXCommand),
    Write(#[defmt(Debug2Format)] WriteXCommand),
    Format(#[defmt(Debug2Format)] FormatCommand),
    SendDiagnostic(#[defmt(Debug2Format)] SendDiagnosticCommand),
    ReportLuns(#[defmt(Debug2Format)] ReportLunsCommand),
    ModeSelect(#[defmt(Debug2Format)] ModeSelectXCommand),
    StartStopUnit(#[defmt(Debug2Format)] StartStopUnitCommand),
    ReadFormatCapacities(#[defmt(Debug2Format)] ReadFormatCapacitiesCommand),
    Verify(#[defmt(Debug2Format)] Verify10Command), // FIXME: Verify16?
    SynchronizeCache(#[defmt(Debug2Format)] SynchronizeCache10Command), // FIXME: SynchronizeCache16?
}

impl Command {
    pub fn extract_from_cbw(cbw: &CommandBlock) -> Result<Command, Error> {
        use num_enum::TryFromPrimitive;

        let op_code =
            OpCode::try_from_primitive(cbw.bytes[0]).map_err(|_| Error::UnhandledOpCode)?;
        match op_code {
            // TODO: return &Command and avoid the copy here
            OpCode::Read6 => Ok(Command::Read((overlay::<Read6Command>(cbw)?).into())),
            OpCode::Read10 => Ok(Command::Read((overlay::<Read10Command>(cbw)?).into())),
            OpCode::Read12 => Ok(Command::Read((overlay::<Read12Command>(cbw)?).into())),
            OpCode::ReadCapacity10 => Ok(Command::ReadCapacity(overlay(cbw)?)),
            OpCode::ReadFormatCapacities => Ok(Command::ReadFormatCapacities(overlay(cbw)?)),
            OpCode::Inquiry => Ok(Command::Inquiry(overlay(cbw)?)),
            OpCode::TestUnitReady => Ok(Command::TestUnitReady(overlay(cbw)?)),
            OpCode::ModeSense6 => Ok(Command::ModeSense(
                (overlay::<ModeSense6Command>(cbw)?).into(),
            )),
            OpCode::ModeSense10 => Ok(Command::ModeSense(
                (overlay::<ModeSense10Command>(cbw)?).into(),
            )),
            OpCode::ModeSelect6 => Ok(Command::ModeSelect(overlay(cbw)?)),
            OpCode::ModeSelect10 => Ok(Command::ModeSelect(overlay(cbw)?)),
            OpCode::PreventAllowMediumRemoval => {
                Ok(Command::PreventAllowMediumRemoval(overlay(cbw)?))
            }
            OpCode::RequestSense => Ok(Command::RequestSense(overlay(cbw)?)),
            OpCode::Write6 => Ok(Command::Write((overlay::<Write6Command>(cbw)?).into())),
            OpCode::Write10 => Ok(Command::Write((overlay::<Write10Command>(cbw)?).into())),
            OpCode::Write12 => Ok(Command::Write((overlay::<Write12Command>(cbw)?).into())),
            OpCode::Format => Ok(Command::Format(overlay(cbw)?)),
            OpCode::SendDiagnostic => Ok(Command::SendDiagnostic(overlay(cbw)?)),
            OpCode::ReportLuns => Ok(Command::ReportLuns(overlay(cbw)?)),
            OpCode::StartStopUnit => Ok(Command::StartStopUnit(overlay(cbw)?)),
            OpCode::Verify10 => Ok(Command::Verify(overlay(cbw)?)),
            OpCode::SynchronizeCache10 => Ok(Command::SynchronizeCache(overlay(cbw)?)),
            _ => Err(Error::UnhandledOpCode),
        }
    }
}

fn overlay<'a, T: overlay::Overlay + Copy>(cbw: &'a CommandBlock) -> Result<T, Error> {
    T::overlay(cbw.bytes).map(|p| *p).map_err(|e| match e {
        overlay::Error::InsufficientLength => Error::InsufficientDataForCommand,
    })
}
