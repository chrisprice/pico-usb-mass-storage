use embassy_usb::driver::Driver;
use embassy_usb::driver::EndpointError;
use embassy_usb::driver::{Endpoint, EndpointIn, EndpointOut};
use embedded_io_async::ErrorType;
use embedded_io_async::Read;

use super::Error;

pub struct Endpoints<'d, D: Driver<'d>> {
    in_ep: D::EndpointIn,
    out_ep: D::EndpointOut,
}

impl<'d, D: Driver<'d>> Endpoints<'d, D> {
    pub fn new(in_ep: D::EndpointIn, out_ep: D::EndpointOut) -> Self {
        assert_eq!(
            in_ep.info().max_packet_size as usize,
            out_ep.info().max_packet_size as usize
        );
        Self { in_ep, out_ep }
    }

    pub fn packet_size(&self) -> usize {
        self.in_ep.info().max_packet_size as usize
    }
}

impl From<EndpointError> for Error {
    fn from(e: EndpointError) -> Self {
        Self::EndpointError(e)
    }
}

impl embedded_io_async::Error for Error {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Self::EndpointError(error) => match error {
                EndpointError::BufferOverflow => embedded_io_async::ErrorKind::OutOfMemory,
                EndpointError::Disabled => embedded_io_async::ErrorKind::NotConnected,
            },
        }
    }
}

impl<'d, D: Driver<'d>> ErrorType for Endpoints<'d, D> {
    type Error = Error;
}

impl<'d, D: Driver<'d>> Read for Endpoints<'d, D> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let count = self.out_ep.read(buf).await?;
        Ok(count)
    }
}

impl<'d, D: Driver<'d>> embedded_io_async::Write for Endpoints<'d, D> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        self.in_ep.write(buf).await?;
        Ok(buf.len())
    }
}
