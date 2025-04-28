use defmt::{debug, error, info};
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_usb::driver::Driver;
use embedded_io_async::ReadExactError;

use crate::{
    bulk_only_transport::{self, BulkOnlyTransport, CommandBlock, CommandError},
    scsi::enums::{AdditionalSenseCode, SenseKey},
    usb_mass_storage::{endpoints::Endpoints, TransportError},
};

use self::{
    commands::*,
    enums::{PageControl, SpcVersion},
    responses::*,
};

mod block_device;
pub use block_device::*;

mod commands;
mod enums;
mod responses;

mod error;
use error::Error;

use self::{
    commands::Command,
    responses::{InquiryResponse, RequestSenseResponse},
};

pub struct Scsi<'d, 'bd, B: Driver<'d>, BD: BlockDevice, M: RawMutex> {
    transport: BulkOnlyTransport<'d, B, M>,
    inquiry_response: InquiryResponse,
    request_sense_response: RequestSenseResponse,
    block_device: &'bd mut BD,
    packet_size: u16,
}

impl<'d, 'bd, B: Driver<'d>, BD: BlockDevice, M: RawMutex> Scsi<'d, 'bd, B, BD, M> {
    /// Creates a new Scsi block device
    ///
    /// `block_device` provides reading and writing of blocks to the underlying filesystem
    ///
    /// `vendor_identification` is an ASCII string that forms part of the SCSI inquiry response.
    ///      Should come from [t10](https://www.t10.org/lists/2vid.htm). Any semi-unique non-blank
    ///      string should work fine for local development. Panics if > 8 characters are supplied.
    ///
    /// `product_identification` is an ASCII string that forms part of the SCSI inquiry response.
    ///      Vendor (probably you...) defined so pick whatever you want. Panics if > 16 characters
    ///      are supplied.
    ///
    /// `product_revision_level` is an ASCII string that forms part of the SCSI inquiry response.
    ///      Vendor (probably you...) defined so pick whatever you want. Typically a version number.
    ///      Panics if > 4 characters are supplied.
    pub fn new(
        endpoints: Endpoints<'d, B, M>,
        block_device: &'bd mut BD,
        vendor_identification: &[u8; 8],
        product_identification: &[u8; 16],
        product_revision_level: &[u8; 4],
        packet_size: u16,
    ) -> Scsi<'d, 'bd, B, BD, M> {
        let mut inquiry_response = InquiryResponse::default();
        inquiry_response.set_vendor_identification(vendor_identification);
        inquiry_response.set_product_identification(product_identification);
        inquiry_response.set_product_revision_level(product_revision_level);

        inquiry_response.set_version(SpcVersion::Spc2); // we are compliant (???)

        Self {
            transport: BulkOnlyTransport::new(endpoints),
            inquiry_response,
            request_sense_response: Default::default(),
            block_device,
            packet_size,
        }
    }

    pub async fn run(&mut self) -> ! {
        let mut handler = BulkHandler {
            block_device: self.block_device,
            inquiry_response: &self.inquiry_response,
            request_sense_response: &mut self.request_sense_response,
            packet_size: self.packet_size,
        };
        self.transport.run(&mut handler).await
    }
}

struct BulkHandler<'scsi, BD> {
    block_device: &'scsi mut BD,
    inquiry_response: &'scsi InquiryResponse,
    request_sense_response: &'scsi mut RequestSenseResponse,
    packet_size: u16,
}

impl<'scsi, BD: BlockDevice> bulk_only_transport::Handler for BulkHandler<'scsi, BD> {
    async fn data_transfer_from_host(
        &mut self,
        cb: &CommandBlock<'_>,
        reader: &mut impl embedded_io_async::Read<Error = TransportError>,
    ) -> Result<(), CommandError> {
        let command = Command::extract_from_cbw(cb).map_err(|e| {
            error!("scsi (from-host) couldn't parse command");
            self.set_sense_from_error(e);
            CommandError::Invalid
        })?;
        info!("scsi from-host command: {}", command);

        match command {
            Command::Write(WriteXCommand {
                lba: lba_start,
                transfer_length,
            }) => {
                let lba_end = lba_start + transfer_length - 1;

                for lba in lba_start..=lba_end {
                    let mut buf = [0u8; 2048];
                    assert!(buf.len() >= BD::BLOCK_BYTES); // TODO: almighty hack
                    let buf = &mut buf[0..BD::BLOCK_BYTES];

                    reader.read_exact(buf).await.map_err(|e| match e {
                        ReadExactError::UnexpectedEof => {
                            error!("Unexpected EOF reading block to write to device");
                            self.set_sense(
                                SenseKey::IllegalRequest,
                                AdditionalSenseCode::InvalidCommandOperationCode,
                            );
                            CommandError::Failed
                        }
                        ReadExactError::Other(e) => CommandError::TransportError(e),
                    })?;

                    self.block_device.write_block(lba, buf).await.map_err(|e| {
                        error!("block device error: {}", e);
                        self.set_sense_from_blockdev_error(e);
                        CommandError::Failed
                    })?;
                }

                Ok(())
            }
            _ => {
                error!("invalid from-host command");
                self.set_sense_invalid_dir();
                Err(CommandError::Invalid)
            }
        }
    }
    async fn data_transfer_to_host(
        &mut self,
        cb: &CommandBlock<'_>,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> Result<(), CommandError> {
        let command = Command::extract_from_cbw(cb).map_err(|e| {
            error!("scsi (to-host) couldn't parse command");
            self.set_sense_from_error(e);
            CommandError::Invalid
        })?;
        info!("scsi to-host command: {}", command);

        match command {
            Command::ReadCapacity(_read_capacity10) => {
                // TODO: support read_capacity16 etc
                let max_lba = self.block_device.block_count();
                let block_size = BD::BLOCK_BYTES as u32;
                let mut cap = ReadCapacity10Response::new();

                cap.set_max_lba(max_lba);
                cap.set_block_size(block_size);

                writer.write_all(cap.as_bytes()).await?;
                Ok(())

                // TODO: readcap16:
                //let mut data = [0u8; 16];
                //let _ = &mut data[0..8].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
                //let _ = &mut data[8..12].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
            }

            Command::Read(ReadXCommand {
                lba: lba_start,
                transfer_length,
            }) => {
                // transfer_length == number of blocks to read
                let lba_end = lba_start + transfer_length - 1;

                // FIXME: what if block_size isn't a multiple of packet_size?
                assert!(
                    BD::BLOCK_BYTES % self.packet_size as usize == 0,
                    "block device's block size must be a multiple of the (usb) packet size (for the current implementation)"
                );

                let mut buf = [0u8; 2048];
                assert!(buf.len() >= BD::BLOCK_BYTES); // TODO: almighty hack
                let buf = &mut buf[0..BD::BLOCK_BYTES];

                for lba in lba_start..=lba_end {
                    self.block_device.read_block(lba, buf).await.map_err(|e| {
                        error!("block device error: {}", e);
                        self.set_sense_from_blockdev_error(e);
                        CommandError::Failed
                    })?;

                    for offset in (0..buf.len()).step_by(self.packet_size as usize) {
                        writer
                            .write_all(&buf[offset..offset + self.packet_size as usize])
                            .await?;
                    }
                }

                Ok(())
            }
            Command::Inquiry { .. } => {
                // FIXME - VPD page should specify maximum transfer_length for read/write
                let buf = &self.inquiry_response.as_bytes()[..InquiryResponse::MINIMUM_SIZE];

                writer.write_all(buf).await?;

                Ok(())
            }
            Command::RequestSense(_) => {
                writer
                    .write_all(self.request_sense_response.as_bytes())
                    .await?;
                Ok(())
            }
            Command::ModeSense(ModeSenseXCommand {
                command_length: CommandLength::C6, // FIXME: handle other mode senses
                page_control: PageControl::CurrentValues,
            }) => {
                let data = [
                    0x03, // number of bytes that follow
                    0x00, // the media type is SBC
                    0x00, // not write-protected, no cache-control bytes support
                    0x00, // no mode-parameter block descriptors
                ];
                writer.write_all(&data).await?;
                Ok(())

                /*
                 * FIXME
                let mut header = ModeParameterHeader6::default();
                header.increase_length_for_page(PageCode::CachingModePage);

                // Default is both caches disabled
                let cache_page = CachingModePage::default();

                let mut buf = [0u8; ModeParameterHeader6::BYTES + CachingModePage::BYTES];

                header.pack(&mut buf[..ModeParameterHeader6::BYTES]).unwrap();
                cache_page.pack(&mut buf[ModeParameterHeader6::BYTES..]).unwrap();
                // FIXME?: original modesense6 response only had 4 bytes, none of this cache_page
                writer.write_all(&buf).await?;
                Ok(())
                */
            }
            Command::ModeSense(_) => todo!(),
            Command::ReadFormatCapacities(ReadFormatCapacitiesCommand { .. }) => {
                let max_lba = self.block_device.block_count();
                let block_size = BD::BLOCK_BYTES as u32;

                let mut response = [0u8; 12];
                response[3] = 0x08; // capacity list length
                response[4..8].copy_from_slice(max_lba.to_be_bytes().as_slice());
                response[8] = 0x02; // formatted media
                response[9..12].copy_from_slice(&block_size.to_be_bytes().as_slice()[1..]); // block size

                writer.write_all(&response).await?;
                Ok(())
            }
            _ => {
                error!("invalid to-host command");
                self.set_sense_invalid_dir();
                Err(CommandError::Invalid)
            }
        }
    }
    async fn no_data_transfer(&mut self, cb: &CommandBlock<'_>) -> Result<(), CommandError> {
        let command = Command::extract_from_cbw(cb).map_err(|e| {
            error!("scsi (no-data) couldn't parse command");
            self.set_sense_from_error(e);
            CommandError::Invalid
        })?;
        debug!("scsi no-data command: {}", command);

        match command {
            Command::PreventAllowMediumRemoval(PreventAllowMediumRemovalCommand { .. }) => {
                // TODO: pass up a level?
                Ok(())
            }
            Command::TestUnitReady(_) => {
                // TODO: after enough errors apparently the host will keep sending TUR
                // requests and nothing else. There may be additional data in the
                // request that indicates we should respond CommandError and prepare
                // sense response data with more info
                Ok(())
            }
            Command::StartStopUnit(StartStopUnitCommand { .. }) => Ok(()),
            Command::Format(_)
            | Command::ModeSelect(_)
            | Command::ReportLuns(_)
            | Command::SendDiagnostic(_)
            | Command::SynchronizeCache(_)
            | Command::Verify(_) => {
                unimplemented!();
            }
            _ => {
                error!("invalid no-data command");
                self.set_sense_invalid_dir();
                Err(CommandError::Invalid)
            }
        }
    }
}

impl<BD> BulkHandler<'_, BD> {
    fn set_sense(&mut self, key: SenseKey, code: AdditionalSenseCode) {
        self.request_sense_response.set_sense_key(key);
        self.request_sense_response.set_additional_sense_code(code);

        info!("sense: set to {}, {}", key, code);
    }

    fn set_sense_from_error(&mut self, e: Error) {
        self.set_sense(
            SenseKey::IllegalRequest,
            match e {
                Error::UnhandledOpCode => AdditionalSenseCode::InvalidCommandOperationCode,
                Error::InsufficientDataForCommand => AdditionalSenseCode::InvalidPacketSize,
                Error::BlockDeviceError(_) => AdditionalSenseCode::WriteError,
            },
        );
    }

    fn set_sense_from_blockdev_error(&mut self, e: BlockDeviceError) {
        match e {
            BlockDeviceError::WriteError => {
                self.set_sense(
                    SenseKey::HardwareError, // or SenseKey::MediumError
                    AdditionalSenseCode::WriteError,
                );
            }
            BlockDeviceError::InvalidAddress => {
                self.set_sense(
                    SenseKey::IllegalRequest,
                    AdditionalSenseCode::LogicalBlockAddressOutOfRange,
                );
            }
        }
    }

    fn set_sense_invalid_dir(&mut self) {
        self.set_sense(
            SenseKey::IllegalRequest,
            AdditionalSenseCode::InvalidCommandOperationCode,
        );
    }
}
