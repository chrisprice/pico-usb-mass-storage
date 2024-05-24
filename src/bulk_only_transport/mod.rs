use core::future::Future;

use defmt::warn;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_usb::driver::Driver;
use embedded_io_async::{Read, ReadExactError, Write};

use crate::usb_mass_storage::{endpoints::Endpoints, TransportError};

use self::{
    cbw::{CommandBlockWrapper, DataDirection, CBW_LEN},
    csw::{build_csw, CommandStatus},
};

pub mod cbw;
pub mod csw;

pub struct CommandBlock<'a> {
    pub bytes: &'a [u8],
    pub lun: u8,
}

pub enum CommandError {
    Failed,
    Invalid,
    TransportError(TransportError),
}

pub trait Handler {
    fn data_transfer_from_host(
        &mut self,
        cb: &CommandBlock,
        reader: &mut impl embedded_io_async::Read<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn data_transfer_to_host(
        &mut self,
        cb: &CommandBlock,
        writer: &mut impl embedded_io_async::Write<Error = TransportError>,
    ) -> impl Future<Output = Result<(), CommandError>>;
    fn no_data_transfer(
        &mut self,
        cb: &CommandBlock,
    ) -> impl Future<Output = Result<(), CommandError>>;
}

pub struct BulkOnlyTransport<'d, D: Driver<'d>, M: RawMutex> {
    endpoints: Endpoints<'d, D, M>,
}

impl<'d, D: Driver<'d>, M: RawMutex> BulkOnlyTransport<'d, D, M> {
    pub fn new(endpoints: Endpoints<'d, D, M>) -> Self {
        Self { endpoints }
    }

    pub async fn run(&mut self, handler: &mut impl Handler) -> ! {
        loop {
            // TODO: the error handling is non-existent here
            let mut buf = [0u8; CBW_LEN];
            match self.endpoints.read_exact(&mut buf).await {
                Ok(_) => {}
                Err(ReadExactError::Other(e)) => {
                    warn!("Transport error reading CBW {}", e);
                    continue;
                }
                Err(ReadExactError::UnexpectedEof) => {
                    warn!("Unexpected EOF reading CBW");
                    continue;
                }
            };
            let cbw = CommandBlockWrapper::from_le_bytes(&buf).unwrap();
            let cb = CommandBlock {
                bytes: &cbw.block[..cbw.block_len],
                lun: cbw.lun,
            };
            let response = match cbw.direction {
                DataDirection::Out => {
                    handler
                        .data_transfer_from_host(&cb, &mut self.endpoints)
                        .await
                }
                DataDirection::In => {
                    handler
                        .data_transfer_to_host(&cb, &mut self.endpoints)
                        .await
                }
                DataDirection::NotExpected => handler.no_data_transfer(&cb).await,
            };
            let status = match response {
                Ok(()) => CommandStatus::Passed,
                Err(CommandError::Failed | CommandError::Invalid) => CommandStatus::Failed,
                Err(CommandError::TransportError(e)) => {
                    warn!("Transport error processing command: {}", e);
                    continue;
                }
            };
            let buf = build_csw(&cbw, status);
            match self.endpoints.write_all(&buf).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("Transport error writing CSW: {}", e);
                    continue;
                }
            }
        }
    }
}
