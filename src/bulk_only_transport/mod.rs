use core::future::Future;

use defmt::info;
use embassy_usb::driver::Driver;
use embedded_io_async::{Read, ReadExactError, Write};

use crate::usb_mass_storage::{endpoints::Endpoints, Error};

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

pub struct CommandFailed;

pub trait Handler {
    fn data_transfer_from_host(
        &mut self,
        cb: &CommandBlock,
        reader: &mut impl embedded_io_async::Read<Error = Error>,
    ) -> impl Future<Output = Result<(), CommandFailed>>;
    fn data_transfer_to_host(
        &mut self,
        cb: &CommandBlock,
        writer: &mut impl embedded_io_async::Write<Error = Error>,
    ) -> impl Future<Output = Result<(), CommandFailed>>;
    fn no_data_transfer(
        &mut self,
        cb: &CommandBlock,
    ) -> impl Future<Output = Result<(), CommandFailed>>;
}

pub struct BulkOnlyTransport<'d, D: Driver<'d>> {
    endpoints: Endpoints<'d, D>,
}

impl<'d, D: Driver<'d>> BulkOnlyTransport<'d, D> {
    pub fn new(endpoints: Endpoints<'d, D>) -> Self {
        Self { endpoints }
    }

    pub async fn run(&mut self, handler: &mut impl Handler) -> ! {
        loop {
            // TODO: the error handling is non-existent here
            let mut buf = [0u8; CBW_LEN];
            if let Err(ReadExactError::UnexpectedEof) = self.endpoints.read_exact(&mut buf).await {
                info!("Unexpected EOF");
                continue;
            }
            let cbw = CommandBlockWrapper::from_le_bytes(&buf).unwrap();
            let cb = CommandBlock {
                bytes: &cbw.block[..cbw.block_len],
                lun: cbw.lun,
            };
            let result = match cbw.direction {
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
            let status = match result {
                Ok(()) => CommandStatus::Passed,
                Err(_) => CommandStatus::Failed,
            };
            let buf = build_csw(&cbw, status);
            self.endpoints.write_all(&buf).await.unwrap();
        }
    }
}
