use defmt::{info, error};
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_usb::driver::Driver;
use embedded_io_async::ReadExactError;
use ::packing::PackedSize;

use crate::{
    bulk_only_transport::{self, BulkOnlyTransport, CommandBlock, CommandError},
    usb_mass_storage::{endpoints::Endpoints, TransportError},
};

use packing::Packed;

use self::{commands::*, enums::{PageControl, SpcVersion}, responses::*};

mod block_device;
pub use block_device::*;

mod commands;
mod responses;
mod enums;
mod packing;

mod error;
use error::Error;

use self::{commands::Command, responses::{InquiryResponse, RequestSenseResponse}};
pub mod command;

pub struct Scsi<'d, B: Driver<'d>, BD: BlockDevice, M: RawMutex> {
    transport: BulkOnlyTransport<'d, B, M>,
    inquiry_response: InquiryResponse,
    request_sense_response: RequestSenseResponse,
    block_device: BD,
    packet_size: u16,
}

impl<'d, B: Driver<'d>, BD: BlockDevice, M: RawMutex> Scsi<'d, B, BD, M> {
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
        block_device: BD,
        vendor_identification: impl AsRef<[u8]>,
        product_identification: impl AsRef<[u8]>,
        product_revision_level: impl AsRef<[u8]>,
        packet_size: u16,
    ) -> Scsi<'d, B, BD, M> {
        let mut inquiry_response = InquiryResponse::default();
        inquiry_response.set_vendor_identification(vendor_identification);
        inquiry_response.set_product_identification(product_identification);
        inquiry_response.set_product_revision_level(product_revision_level);

        inquiry_response.version = SpcVersion::Spc2; // we are compliant (???)

        //TODO: This is reasonable for FAT but not FAT32 or others. BOT buffer should probably be
        //configurable from here, perhaps passing in BD::BLOCK_BYTES.max(BOT::MIN_BUFFER) or something
        //assert!(BD::BLOCK_BYTES <= BulkOnlyTransport::<B, M>::BUFFER_BYTES);
        // FIXME

        Self {
            transport: BulkOnlyTransport::new(endpoints),
            inquiry_response,
            request_sense_response: Default::default(),
            block_device,
            packet_size,
        }
    }

    /// Grants access to the block device for the purposes of housekeeping etc.
    pub fn block_device_mut(&mut self) -> &mut BD {
        &mut self.block_device
    }

    pub async fn run(&mut self) {
        let mut handler = BulkHandler {
            block_device: &mut self.block_device,
            inquiry_response: &self.inquiry_response,
            request_sense_response: &mut self.request_sense_response,
            packet_size: self.packet_size,
        };
        self.transport.run(&mut handler).await;
    }
}

struct BulkHandler<'scsi, BD: BlockDevice> {
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
        let command = Command::extract_from_cbw(cb).map_err(|_| {
            // TODO: better details / split apart the error type
            error!("scsi (from-host) couldn't parse, first byte {}", cb.bytes[0]);
            CommandError::CommandInvalid
        })?;
        info!("scsi from-host command: {}", command);

        match command {
            Command::Write(WriteXCommand { lba: lba_start, transfer_length }) => {
                // Record the end condition
                let lba_end = lba_start + transfer_length - 1;

                // trace_scsi_fs!("FS> Read; new: {}, lba: 0x{:X?}, lba_end: 0x{:X?}, done: {}",
                //     new_command, self.lba, self.lba_end, self.lba == self.lba_end);

                for lba in lba_start..=lba_end {
                    let mut buf = [0u8; 2048];
                    assert!(buf.len() >= BD::BLOCK_BYTES); // TODO: almighty hack
                    let buf = &mut buf[0..BD::BLOCK_BYTES];

                    reader.read_exact(buf).await
                        .map_err(|e| match e {
                            ReadExactError::UnexpectedEof => {
                                panic!("Unexpected EOF reading block to write to device")
                            }
                            ReadExactError::Other(e) => e,
                        })?;

                    self.block_device.write_block(lba, buf)
                        .map_err(|_e| /*TODO: log e*/CommandError::CommandFailed)?;
                }

                Ok(())
            }
            Command::Read { .. }
            | Command::Inquiry { .. }
            | Command::TestUnitReady(_)
            | Command::RequestSense(_)
            | Command::ModeSense(_)
            | Command::ReadFormatCapacities { .. }
            | Command::ReadCapacity(_)
            | Command::PreventAllowMediumRemoval(_)
            | Command::Format(_)
            | Command::SendDiagnostic(_)
            | Command::ReportLuns(_)
            | Command::ModeSelect(_)
            | Command::StartStopUnit(_)
            | Command::Verify(_)
            | Command::SynchronizeCache(_) => {
                todo!();
                //self.request_sense_response.sense_key.set(SenseKey::IllegalRequest);
                //STATE.sense_key.replace(0x05); // illegal request Sense Key
                //STATE.sense_key_code.replace(0x20); // Invalid command operation ASC
                //STATE.sense_qualifier.replace(0x00); // Invalid command operation ASCQ
            }
        }
    }
    async fn data_transfer_to_host(
        &mut self,
        cb: &CommandBlock<'_>,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> Result<(), CommandError> {
        let command = Command::extract_from_cbw(cb).map_err(|_| {
            // TODO: better details / split apart the error type
            error!("scsi (to-host) couldn't parse, first byte {}", cb.bytes[0]);
            CommandError::CommandInvalid
        })?;
        info!("scsi to-host command: {}", command);

        match command {
            Command::ReadCapacity(_read_capacity10) => {
                // TODO: support read_capacity16 etc
                let max_lba = self.block_device.max_lba();
                let block_size = BD::BLOCK_BYTES as u32;
                let cap = ReadCapacity10Response {
                    max_lba,
                    block_size,
                };

                let mut buf = [0u8; ReadCapacity10Response::BYTES];
                cap.pack(&mut buf).unwrap();
                writer.write_all(&buf).await?;
                Ok(())

                // TODO: readcap16:
                //let mut data = [0u8; 16];
                //let _ = &mut data[0..8].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
                //let _ = &mut data[8..12].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
            },

            Command::Read(ReadXCommand { lba: lba_start, transfer_length }) => {
                // transfer_length == number of blocks to read
                let lba_end = lba_start + transfer_length - 1;

                // trace_scsi_fs!("FS> Read; new: {}, lba: 0x{:X?}, lba_end: 0x{:X?}, done: {}",
                //     new_command, self.lba, self.lba_end, self.lba == self.lba_end);

                // FIXME: what if block_size isn't a multiple of packet_size?
                assert!(
                    BD::BLOCK_BYTES % self.packet_size as usize == 0,
                    "block device's block size must be a multiple of the (usb) packet size (for the current implementation)"
                );

                // FIXME: if lba+transfer_length*block_size is out-of-bounds:
                // sense = IllegalRequest
                // sense code = lba_out_of_range

                let mut buf = [0u8; 2048];
                assert!(buf.len() >= BD::BLOCK_BYTES); // TODO: almighty hack
                let buf = &mut buf[0..BD::BLOCK_BYTES];

                for lba in lba_start..=lba_end {
                    self.block_device.read_block(lba, buf)
                        .map_err(|_e| /*TODO: log e*/CommandError::CommandFailed)?;

                    for offset in (0..buf.len()).step_by(self.packet_size as usize) {
                        writer
                            .write_all(&buf[offset..offset + self.packet_size as usize])
                            .await?;
                    }
                }

                Ok(())
            },
            Command::Inquiry { .. } => {
                // FIXME - VPD page should specify maximum transfer_length for read/write
                let mut buf = [0u8; InquiryResponse::BYTES];

                self.inquiry_response.pack(&mut buf).unwrap();

                writer.write_all(&buf[..InquiryResponse::MINIMUM_SIZE + self.inquiry_response.additional_length as usize]).await?;

                Ok(())
            }
            Command::RequestSense(_) => {
                let mut buf = [0u8; RequestSenseResponse::BYTES];
                self.request_sense_response.pack(&mut buf).unwrap();
                writer.write_all(&buf).await?;
                Ok(())
            },
            Command::ModeSense(ModeSenseXCommand {
                command_length: CommandLength::C6, // FIXME: handle other mode senses
                page_control: PageControl::CurrentValues,
            })  => {
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
            },
            Command::ModeSense(_) => todo!(),
            Command::ReadFormatCapacities(ReadFormatCapacitiesCommand {
                ..
            }) => {
                //let mut data = [0u8; 12];
                //let _ = &mut data[0..4].copy_from_slice(&[
                //    0x00, 0x00, 0x00, 0x08, // capacity list length
                //]);
                //let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCKS)); // number of blocks
                //data[8] = 0x01; //unformatted media
                //let block_length_be = u32::to_be_bytes(BLOCK_SIZE);
                //data[9] = block_length_be[1];
                //data[10] = block_length_be[2];
                //data[11] = block_length_be[3];
                todo!()
            }
            Command::Format(_) => todo!(),
            Command::SendDiagnostic(_) => todo!(),
            Command::ReportLuns(_) => todo!(),
            Command::ModeSelect(_) => todo!(),
            Command::StartStopUnit(_) => todo!(),
            Command::Verify(_) => todo!(),
            Command::SynchronizeCache(_) => todo!(),

            Command::PreventAllowMediumRemoval(_) | Command::TestUnitReady(_) | Command::Write(_) => {
                panic!("unexepected direction")
            }
        }
    }
    async fn no_data_transfer(&mut self, cb: &CommandBlock<'_>) -> Result<(), CommandError> {
        let command = Command::extract_from_cbw(cb).map_err(|_| {
            // TODO: better details / split apart the error type
            error!("scsi (no-data) couldn't parse, first byte {}", cb.bytes[0]);
            CommandError::CommandInvalid
        })?;
        info!("scsi no-data command: {}", command);

        match command {
            Command::PreventAllowMediumRemoval(PreventAllowMediumRemovalCommand { prevent: _prevent, .. }) => {
                // TODO: pass up a level?
                Ok(())
            }
            Command::TestUnitReady(_) => {
                // TODO: after enough errors apparently the host will keep sending TUR
                // requests and nothing else. There may be additional data in the
                // request that indicates we should respond CommandError and prepare
                // sense response data with more info
                Ok(())
            },
            _ => {
                panic!("unexepected direction {:?}", command)
            }
        }
    }
}
