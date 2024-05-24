use embassy_futures::select::select;
use embassy_futures::select::Either;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::signal::Signal;
use embassy_usb::driver::Driver;
use embassy_usb::driver::EndpointError;
use embassy_usb::driver::{Endpoint, EndpointIn, EndpointOut};
use embedded_io_async::ErrorType;
use embedded_io_async::Read;

use super::TransportError;

pub struct Endpoints<'d, D: Driver<'d>, M: RawMutex> {
    in_ep: D::EndpointIn,
    out_ep: D::EndpointOut,
    reset_signal: &'d Signal<M, ()>,
}

impl<'d, D: Driver<'d>, M: RawMutex> Endpoints<'d, D, M> {
    pub fn new(
        in_ep: D::EndpointIn,
        out_ep: D::EndpointOut,
        reset_signal: &'d Signal<M, ()>,
    ) -> Self {
        assert_eq!(
            in_ep.info().max_packet_size,
            out_ep.info().max_packet_size
        );
        Self {
            in_ep,
            out_ep,
            reset_signal,
        }
    }
}

impl From<EndpointError> for TransportError {
    fn from(e: EndpointError) -> Self {
        Self::Endpoint(e)
    }
}

impl embedded_io_async::Error for TransportError {
    fn kind(&self) -> embedded_io_async::ErrorKind {
        match self {
            Self::Endpoint(error) => match error {
                EndpointError::BufferOverflow => embedded_io_async::ErrorKind::OutOfMemory,
                EndpointError::Disabled => embedded_io_async::ErrorKind::NotConnected,
            },
            Self::Reset() => embedded_io_async::ErrorKind::Other,
        }
    }
}

impl<'d, D: Driver<'d>, M: RawMutex> ErrorType for Endpoints<'d, D, M> {
    type Error = TransportError;
}

impl<'d, D: Driver<'d>, M: RawMutex> Read for Endpoints<'d, D, M> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let read_future = self.out_ep.read(buf);
        let reset_future = self.reset_signal.wait();
        match select(read_future, reset_future).await {
            Either::First(read_result) => match read_result {
                Ok(count) => Ok(count),
                Err(e) => Err(e.into()),
            },
            Either::Second(_) => Err(TransportError::Reset()),
        }
    }
}

impl<'d, D: Driver<'d>, M: RawMutex> embedded_io_async::Write for Endpoints<'d, D, M> {
    async fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let write_future = self.in_ep.write(buf);
        let reset_future = self.reset_signal.wait();
        match select(write_future, reset_future).await {
            Either::First(write_result) => match write_result {
                Ok(()) => Ok(buf.len()),
                Err(e) => Err(e.into()),
            },
            Either::Second(()) => Err(TransportError::Reset()),
        }
    }
}
