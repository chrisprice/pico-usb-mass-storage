use core::future::Future;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_usb::driver::Driver;

use crate::{
    bulk_only_transport::{self, BulkOnlyTransport, CommandBlock, CommandError},
    scsi::command::ScsiCommand,
    usb_mass_storage::{endpoints::Endpoints, TransportError},
};

use self::command::{parse_cb, PageControl};
pub mod command;

pub trait Handler {
    fn read(
        &mut self,
        lba: u64,
        len: u64,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn write(
        &mut self,
        lba: u64,
        len: u64,
        reader: &mut impl embedded_io_async::Read<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn inquiry(
        &mut self,
        evpd: bool,
        page_code: u8,
        alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn test_unit_ready(&mut self) -> impl Future<Output = Result<(), CommandError>>;
    fn request_sense(
        &mut self,
        desc: bool,
        alloc_len: u8,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn mode_sense6(
        &mut self,
        dbd: bool,
        page_control: PageControl,
        page_code: u8,
        subpage_code: u8,
        alloc_len: u8,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn mode_sense10(
        &mut self,
        dbd: bool,
        page_control: PageControl,
        page_code: u8,
        subpage_code: u8,
        alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn read_capacity10(
        &mut self,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn read_capacity16(
        &mut self,
        alloc_len: u32,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn read_format_capacities(
        &mut self,
        alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn unknown(&mut self) -> impl Future<Output = Result<(), CommandError>>;
}

pub struct Scsi<'d, D: Driver<'d>, M: RawMutex> {
    transport: BulkOnlyTransport<'d, D, M>,
}

impl<'d, D: Driver<'d>, M: RawMutex> Scsi<'d, D, M> {
    pub fn new(endpoints: Endpoints<'d, D, M>) -> Self {
        Self {
            transport: BulkOnlyTransport::new(endpoints),
        }
    }

    pub async fn run(&mut self, handler: &mut impl Handler) {
        let mut adapter = Adapter { handler };
        self.transport.run(&mut adapter).await;
    }
}

struct Adapter<'h, H: Handler> {
    handler: &'h mut H,
}

impl<'h, H: Handler> bulk_only_transport::Handler for Adapter<'h, H> {
    async fn data_transfer_from_host(
        &mut self,
        cb: &CommandBlock<'_>,
        reader: &mut impl embedded_io_async::Read<Error = TransportError>,
    ) -> Result<(), CommandError> {
        match parse_cb(cb.bytes) {
            ScsiCommand::Write { lba, len } => self.handler.write(lba, len, reader).await,
            ScsiCommand::Read { .. }
            | ScsiCommand::Unknown
            | ScsiCommand::Inquiry { .. }
            | ScsiCommand::TestUnitReady
            | ScsiCommand::RequestSense { .. }
            | ScsiCommand::ModeSense6 { .. }
            | ScsiCommand::ModeSense10 { .. }
            | ScsiCommand::ReadCapacity10
            | ScsiCommand::ReadCapacity16 { .. }
            | ScsiCommand::ReadFormatCapacities { .. } => todo!("unexepected direction"),
        }
    }
    async fn data_transfer_to_host(
        &mut self,
        cb: &CommandBlock<'_>,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> Result<(), CommandError> {
        match parse_cb(cb.bytes) {
            ScsiCommand::Read { lba, len } => self.handler.read(lba, len, writer).await,
            ScsiCommand::Unknown => self.handler.unknown().await,
            ScsiCommand::Inquiry {
                evpd,
                page_code,
                alloc_len,
            } => {
                self.handler
                    .inquiry(evpd, page_code, alloc_len, writer)
                    .await
            }
            ScsiCommand::RequestSense { desc, alloc_len } => {
                self.handler.request_sense(desc, alloc_len, writer).await
            }
            ScsiCommand::ModeSense6 {
                dbd,
                page_control,
                page_code,
                subpage_code,
                alloc_len,
            } => {
                self.handler
                    .mode_sense6(
                        dbd,
                        page_control,
                        page_code,
                        subpage_code,
                        alloc_len,
                        writer,
                    )
                    .await
            }
            ScsiCommand::ModeSense10 {
                dbd,
                page_control,
                page_code,
                subpage_code,
                alloc_len,
            } => {
                self.handler
                    .mode_sense10(
                        dbd,
                        page_control,
                        page_code,
                        subpage_code,
                        alloc_len,
                        writer,
                    )
                    .await
            }
            ScsiCommand::ReadCapacity10 => self.handler.read_capacity10(writer).await,
            ScsiCommand::ReadCapacity16 { alloc_len } => {
                self.handler.read_capacity16(alloc_len, writer).await
            }
            ScsiCommand::ReadFormatCapacities { alloc_len } => {
                self.handler.read_format_capacities(alloc_len, writer).await
            }
            ScsiCommand::TestUnitReady | ScsiCommand::Write { .. } => {
                todo!("unexepected direction")
            }
        }
    }
    async fn no_data_transfer(&mut self, cb: &CommandBlock<'_>) -> Result<(), CommandError> {
        let command = parse_cb(cb.bytes);
        match command {
            ScsiCommand::TestUnitReady => self.handler.test_unit_ready().await,
            ScsiCommand::Unknown => Err(CommandError::CommandFailed),
            ScsiCommand::Read { .. }
            | ScsiCommand::Write { .. }
            | ScsiCommand::Inquiry { .. }
            | ScsiCommand::RequestSense { .. }
            | ScsiCommand::ModeSense6 { .. }
            | ScsiCommand::ModeSense10 { .. }
            | ScsiCommand::ReadCapacity10
            | ScsiCommand::ReadCapacity16 { .. }
            | ScsiCommand::ReadFormatCapacities { .. } => {
                todo!("unexepected direction {:?}", command)
            }
        }
    }
}
