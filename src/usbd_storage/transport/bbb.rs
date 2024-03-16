//! Bulk Only Transport (BBB/BOT)

use crate::usbd_storage::buffer::{AsyncReader, AsyncWriter, Buffer};
use crate::usbd_storage::transport::{CommandStatus, Transport, TransportError};
use core::borrow::BorrowMut;
use core::cell::Cell;
use core::cmp::min;
use core::mem::MaybeUninit;
use defmt::{info, trace, Format};
use embassy_usb::control::{InResponse, Recipient, Request, RequestType};
use embassy_usb::driver::{Driver, Endpoint, EndpointIn, EndpointOut};
use embassy_usb::{Builder, Handler};
use ringbuffer::ConstGenericRingBuffer;

/// Bulk Only Transport interface protocol
pub(crate) const TRANSPORT_BBB: u8 = 0x50;

const CLASS_SPECIFIC_BULK_ONLY_MASS_STORAGE_RESET: u8 = 0xFF;
const CLASS_SPECIFIC_GET_MAX_LUN: u8 = 0xFE;

const CBW_SIGNATURE_LE: [u8; 4] = 0x43425355u32.to_le_bytes();
const CSW_SIGNATURE_LE: [u8; 4] = 0x53425355u32.to_le_bytes();

const CBW_LEN: usize = 31;
const CSW_LEN: usize = 13;

struct InvalidCbwError; // Inner transport-specific error

/// Bulk Only Transport error
#[derive(Debug, Format)]
pub enum BulkOnlyError {
    /// Not enough space to fit additional data
    IoBufferOverflow,
    /// Invalid MAX_LUN value. Refer to USB BBB doc
    InvalidMaxLun,
    /// Transport is not in Data Transfer state
    InvalidState,
    /// Data Transfer expects a full packet to be sent next but not enough data available
    FullPacketExpected,
    /// The IO buffer cannot fit a CBW or a single full packet
    BufferTooSmall,
}

/// Raw Command Block bytes
///
/// The `bytes` field is a truncated slice
pub struct CommandBlock<'a> {
    pub bytes: &'a [u8],
    pub lun: u8,
}

#[derive(Debug, Copy, Clone, Format)]
enum State {
    Idle,                 // no active transfer
    CommandTransfer,      // reading CBW packets
    DataTransferToHost,   // writing bytes to host
    DataTransferFromHost, // reading bytes from host
    DataTransferNoData,   // data transfer not expected
    Reset,                // the bus has been reset
}

#[repr(u8)]
#[derive(Default, Debug, Copy, Clone, Format)]
enum DataDirection {
    Out,
    In,
    #[default]
    NotExpected,
}

type BulkOnlyTransportResult<T> = Result<T, TransportError<BulkOnlyError>>;

pub struct StateHarder<'a, const CAP: usize> {
    state: Cell<State>,
    buffer: ConstGenericRingBuffer<u8, CAP>,
    control: MaybeUninit<Control<'a>>,
}

impl<'a, const CAP: usize> Default for StateHarder<'a, CAP> {
    fn default() -> Self {
        Self {
            state: Cell::new(State::Idle),
            buffer: ConstGenericRingBuffer::new(),
            control: MaybeUninit::uninit(),
        }
    }
}

pub struct Control<'a> {
    state: &'a Cell<State>,
    max_lun: u8,
}

/// Bulk Only Transport
///
/// Expected to be driven via [write] and [read] methods.
/// All data goes through an underlying IO buffer in both directions.
/// During a Data Transfer, data could be read or written via [read_data], [write_data]
/// and [try_write_data_all] methods.
///
/// [write]: crate::transport::bbb::BulkOnly::write
/// [read]: crate::transport::bbb::BulkOnly::read
/// [read_data]: crate::transport::bbb::BulkOnly::read_data
/// [write_data]: crate::transport::bbb::BulkOnly::write_data
/// [try_write_data_all]: crate::transport::bbb::BulkOnly::try_write_data_all
pub struct BulkOnly<'d, D: Driver<'d>, Buf: BorrowMut<[u8]>> {
    in_ep: D::EndpointIn,
    out_ep: D::EndpointOut,
    state: &'d Cell<State>,
    buf: Buffer<Buf>,
    cbw: CommandBlockWrapper,
    cs: Option<CommandStatus>,
}

impl<'d, D, Buf> BulkOnly<'d, D, Buf>
where
    D: Driver<'d>,
    Buf: BorrowMut<[u8]>,
{
    /// Creates a Bulk Only Transport instance
    ///
    /// # Arguments
    /// * `alloc` - [UsbDAllocator]
    /// * `packet_size` - Maximum USB packet size. Allowed values: 8,16,32,64
    /// * `max_lun` - The max index of the Logical Unit
    /// * `buf` - The underlying IO buffer. It is **required** to fit at least a `CBW` and/or a single
    /// packet. It is **recommended** that buffer fits at least one `LBA` size
    ///
    /// # Errors
    /// * [InvalidMaxLun]
    /// * [BufferTooSmall]
    ///
    /// # Panics
    /// Panics if endpoint allocations fails.
    ///
    /// [InvalidMaxLun]: crate::transport::bbb::BulkOnlyError::InvalidMaxLun
    /// [BufferTooSmall]: crate::transport::bbb::BulkOnlyError::BufferTooSmall
    /// [UsbDAllocator]: usb_device::bus::UsbDAllocator
    pub fn new<'b, const CAP: usize>(
        builder: &'b mut Builder<'d, D>,
        in_ep: D::EndpointIn,
        out_ep: D::EndpointOut,
        state: &'d mut StateHarder<'d, CAP>,
        buf: Buf,
        max_lun: u8,
    ) -> Result<BulkOnly<'d, D, Buf>, BulkOnlyError> {
        if max_lun > 0x0F {
            return Err(BulkOnlyError::InvalidMaxLun);
        }

        let buf_len = buf.borrow().len();
        let packet_size = in_ep.info().max_packet_size as usize;
        assert_eq!(packet_size, out_ep.info().max_packet_size as usize);
        if buf_len < CBW_LEN || buf_len < packet_size {
            return Err(BulkOnlyError::BufferTooSmall);
        }

        let control = state.control.write(Control {
            state: &state.state,
            max_lun,
        });

        builder.handler(control);

        Ok(BulkOnly {
            in_ep,
            out_ep,
            buf: Buffer::new(buf),
            cbw: Default::default(),
            cs: Default::default(),
            state: &state.state,
        })
    }

    /// Drives a transport by reading a single packet
    pub async fn read(&mut self) -> BulkOnlyTransportResult<()> {
        match self.state.get() {
            State::Idle | State::CommandTransfer => self.handle_read_cbw().await,
            State::DataTransferFromHost => self.handle_read_from_host().await,
            _ => Ok(()),
        }
    }

    /// Drives a transport by writing a single packet
    pub async fn write(&mut self) -> BulkOnlyTransportResult<()> {
        match self.state.get() {
            State::DataTransferToHost => self.handle_write_to_host().await,
            State::DataTransferNoData => self.handle_no_data_transfer().await,
            _ => Ok(()),
        }
    }

    /// Sets a `status` of the current command
    ///
    /// This method doesn't try to send a status immediately. However, all further
    /// writes to the IO buffer won't succeed. The transport will try to send all
    /// the contents of the buffer and then `CSW` will be sent.
    ///
    /// # Panics
    /// Panics if called during any by Data Transfer state. Usually, this means an error in
    /// class implementation.
    pub fn set_status(&mut self, status: CommandStatus) {
        assert!(matches!(
            self.state.get(),
            State::DataTransferToHost | State::DataTransferFromHost | State::DataTransferNoData
        ));
        info!("usb: bbb: Set status: {}", status);
        self.cs = Some(status);
    }

    /// Returns a Command Block if present
    pub fn get_command(&self) -> Option<CommandBlock> {
        match self.state.get() {
            State::Idle | State::CommandTransfer => None,
            _ => Some(CommandBlock {
                bytes: &self.cbw.block[..self.cbw.block_len],
                lun: self.cbw.lun,
            }),
        }
    }

    /// Reads data from the IO buffer returning the number of bytes actually read
    ///
    /// # Arguments
    /// * `dst` - buffer, to read bytes into
    ///
    /// # Errors
    /// Returns [BulkOnlyError::InvalidState] if called
    /// during any but OUT Data Transfer state.
    ///
    /// [BulkOnlyError::InvalidState]: crate::transport::bbb::BulkOnlyError::InvalidState
    pub fn read_data(&mut self, dst: &mut [u8]) -> BulkOnlyTransportResult<usize> {
        if !matches!(self.state.get(), State::DataTransferFromHost) {
            return Err(TransportError::Error(BulkOnlyError::InvalidState));
        }
        Ok(self
            .buf
            .read(|buf| {
                let size = min(dst.len(), buf.len());
                dst[..size].copy_from_slice(buf);
                Ok::<usize, ()>(size)
            })
            .unwrap())
    }

    /// Writes data from the IO buffer returning the number of bytes actually written
    ///
    /// # Arguments
    /// * `src` - bytes to write
    ///
    /// # Errors
    /// Returns [BulkOnlyError::InvalidState] if called
    /// during any but IN Data Transfer state.
    ///
    /// [BulkOnlyError::InvalidState]: crate::transport::bbb::BulkOnlyError::InvalidState
    pub fn write_data(&mut self, src: &[u8]) -> BulkOnlyTransportResult<usize> {
        if !matches!(self.state.get(), State::DataTransferToHost) {
            return Err(TransportError::Error(BulkOnlyError::InvalidState));
        }
        if !self.status_present() {
            let len = self.cbw.data_transfer_len as usize;
            Ok(self.buf.write(&src[..min(src.len(), len)]))
        } else {
            Err(TransportError::Error(BulkOnlyError::InvalidState))
        }
    }

    /// Tries to write all data from `src` into the IO buffer returning the number of bytes actually written
    ///
    /// # Errors
    /// * [BulkOnlyError::IoBufferOverflow] - if not enough space is available
    /// * [BulkOnlyError::InvalidState] - if called during any but IN Data Transfer state
    ///
    /// [BulkOnlyError::IoBufferOverflow]: crate::transport::bbb::BulkOnlyError::IoBufferOverflow
    /// [BulkOnlyError::InvalidState]: crate::transport::bbb::BulkOnlyError::InvalidState
    pub fn try_write_data_all(&mut self, src: &[u8]) -> BulkOnlyTransportResult<()> {
        if !matches!(self.state.get(), State::DataTransferToHost) {
            return Err(TransportError::Error(BulkOnlyError::InvalidState));
        }
        if !self.status_present() {
            self.buf
                .write_all(
                    src.len(),
                    TransportError::Error(BulkOnlyError::IoBufferOverflow),
                    |dst| {
                        dst[..src.len()].copy_from_slice(src);
                        Ok(src.len())
                    },
                )
                .map(|_| ())
        } else {
            Err(TransportError::Error(BulkOnlyError::InvalidState))
        }
    }

    /// Whether a Command Status has been set
    pub fn has_status(&self) -> bool {
        self.status_present()
    }

    async fn handle_read_cbw(&mut self) -> BulkOnlyTransportResult<()> {
        while self.buf.available_read() < CBW_LEN {
            self.read_packet().await?; // propagate if error or WouldBlock
        }
        if self.buf.available_read() >= CBW_LEN {
            // try parse CBW if enough data available
            match self.try_parse_cbw() {
                Ok(cbw) => {
                    info!("usb: bbb: Recv CBW: {}", cbw);
                    self.start_data_transfer(cbw);
                }
                Err(_) => {
                    // Spec. 6.6.1
                    self.stall_eps();
                    self.enter_state(State::Idle);
                }
            }
        } else {
            // we've read something but it's not enough yet
            self.enter_state(State::CommandTransfer)
        }
        Ok(())
    }

    async fn handle_read_from_host(&mut self) -> BulkOnlyTransportResult<()> {
        if !self.status_present() && self.cbw.data_transfer_len > 0 {
            let count = self.read_packet().await?; // propagate if error or WouldBlock
            self.cbw.data_transfer_len = self.cbw.data_transfer_len.saturating_sub(count as u32);
            trace!("usb: bbb: Data residue: {}", self.cbw.data_transfer_len);
        }
        self.check_end_data_transfer().await
    }

    async fn handle_write_to_host(&mut self) -> BulkOnlyTransportResult<()> {
        // Do not send a short packet if there is not enough data in the buffer. Some drivers
        // consider this as an error.
        // If the next packet is expected to be full (according to data residue) but it isn't,
        // return an error

        let max_packet_size = self.packet_size() as u32;
        let full_packet_expected =
            self.cbw.data_transfer_len >= max_packet_size && !self.status_present();
        let full_packet = self.buf.available_read() >= max_packet_size as usize;
        let full_packet_or_zero = full_packet || !full_packet_expected;

        if full_packet_or_zero {
            // attempt to send data from buffer if any
            if self.buf.available_read() > 0 {
                let count = self.write_packet().await?; // propagate if error
                self.cbw.data_transfer_len =
                    self.cbw.data_transfer_len.saturating_sub(count as u32);
                trace!("usb: bbb: Data residue: {}", self.cbw.data_transfer_len);
            }
            self.check_end_data_transfer().await
        } else {
            Err(TransportError::Error(BulkOnlyError::FullPacketExpected))
        }
    }

    async fn handle_no_data_transfer(&mut self) -> BulkOnlyTransportResult<()> {
        self.check_end_data_transfer().await
    }

    async fn handle_write_csw(&mut self) -> BulkOnlyTransportResult<()> {
        self.write_packet().await?; // propagate if error
        if self.buf.available_read() == 0 {
            self.enter_state(State::Idle) // done with status transfer
        }
        Ok(())
    }

    async fn check_end_data_transfer(&mut self) -> BulkOnlyTransportResult<()> {
        if self.status_present() {
            let state = self.state.get();
            match state {
                State::DataTransferNoData => {
                    self.end_data_transfer().await?;
                }
                State::DataTransferFromHost | State::DataTransferToHost
                    if self.buf.available_read() == 0 =>
                {
                    self.end_data_transfer().await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn end_data_transfer(&mut self) -> BulkOnlyTransportResult<()> {
        // spec. 6.7.2 and 6.7.3
        if self.cbw.data_transfer_len > 0 {
            match self.state.get() {
                State::DataTransferToHost => {
                    //TODO: send zlp right here
                    self.stall_in_ep();
                }
                State::DataTransferFromHost => {
                    self.stall_out_ep();
                }
                _ => {}
            }
        }

        // write CSW into buffer
        let csw = self.build_csw().unwrap();
        self.buf.clean();
        self.buf.write(csw.as_slice());

        self.handle_write_csw().await
    }

    #[inline]
    fn status_present(&self) -> bool {
        self.cs.is_some()
    }

    fn build_csw(&mut self) -> Option<[u8; CSW_LEN]> {
        self.cs.map(|status| {
            let mut csw = [0u8; CSW_LEN];
            csw[..4].copy_from_slice(CSW_SIGNATURE_LE.as_slice());
            csw[4..8].copy_from_slice(self.cbw.tag.to_le_bytes().as_slice());
            csw[8..12].copy_from_slice(self.cbw.data_transfer_len.to_le_bytes().as_slice());
            csw[12..].copy_from_slice(&[status as u8]);
            csw
        })
    }

    /// The caller must ensure that there is enough data available
    fn try_parse_cbw(&mut self) -> Result<CommandBlockWrapper, InvalidCbwError> {
        debug_assert!(matches!(
            self.state.get(),
            State::Idle | State::CommandTransfer
        ));
        debug_assert!(self.buf.available_read() >= CBW_LEN);

        // read CBW from buf
        let mut raw_cbw = [0u8; CBW_LEN];
        self.buf
            .read::<()>(|buf| {
                raw_cbw.copy_from_slice(&buf[..CBW_LEN]); // buf.len() checked in the beginning
                Ok(CBW_LEN)
            })
            .unwrap();

        // check if CBW is valid. Spec. 6.2.1
        if !raw_cbw.starts_with(&CBW_SIGNATURE_LE) {
            return Err(InvalidCbwError);
        }

        CommandBlockWrapper::from_le_bytes(&raw_cbw[4..]) // parse CBW (skipping signature)
    }

    fn start_data_transfer(&mut self, mut cbw: CommandBlockWrapper) {
        debug_assert!(matches!(
            self.state.get(),
            State::Idle | State::CommandTransfer
        ));

        // build new state
        match cbw.direction {
            DataDirection::Out => {
                self.enter_state(State::DataTransferFromHost);
            }
            DataDirection::In => {
                self.enter_state(State::DataTransferToHost);
            }
            DataDirection::NotExpected => {
                self.enter_state(State::DataTransferNoData);
                cbw.data_transfer_len = 0; // original value ignored
            }
        };
        self.cbw = cbw;
    }

    #[inline]
    fn packet_size(&self) -> usize {
        self.in_ep.info().max_packet_size as usize // same for both In and Out EPs
    }

    async fn read_packet(&mut self) -> BulkOnlyTransportResult<usize> {
        // if let State::Reset = self.state.get() {
        //     self.enter_state(State::Idle);
        //     return Err(TransportError::Error(BulkOnlyError::InvalidState));
        // }
        let count = self
            .buf
            .write_all_async(
                self.packet_size(),
                TransportError::Error(BulkOnlyError::IoBufferOverflow),
                OutEndPointReader::<'_, '_, D>(&mut self.out_ep),
            )
            .await?;
        if let State::Reset = self.state.get() {
            self.enter_state(State::Idle);
            return Err(TransportError::Error(BulkOnlyError::InvalidState));
        }
        trace!(
            "usb: bbb: Read bytes: {}, buf available: {}",
            count,
            self.buf.available_read()
        );

        Ok(count)
    }

    /// Write single packet from [buf] returning number of bytes actually written
    async fn write_packet(&mut self) -> BulkOnlyTransportResult<usize> {
        // if let State::Reset = self.state.get() {
        //     self.enter_state(State::Idle);
        //     return Err(TransportError::Error(BulkOnlyError::InvalidState));
        // }
        let count = self
            .buf
            .read_async(InEndPointWriter::<'_, '_, D>(&mut self.in_ep))
            .await?;
        if let State::Reset = self.state.get() {
            self.enter_state(State::Idle);
            return Err(TransportError::Error(BulkOnlyError::InvalidState));
        }
        trace!(
            "usb: bbb: Wrote bytes: {}, buf available: {}",
            count,
            self.buf.available_read()
        );

        Ok(count)
    }

    #[inline]
    fn stall_eps(&self) {
        self.stall_in_ep();
        self.stall_out_ep();
    }

    #[inline]
    fn stall_in_ep(&self) {
        todo!("usb: bbb: Stall IN ep");
        // info!("usb: bbb: Stall IN ep");
        // self.in_ep.stall();
    }

    #[inline]
    fn stall_out_ep(&self) {
        todo!("usb: bbb: Stall OUT ep");
        // info!("usb: bbb: Stall OUT ep");
        // self.out_ep.stall();
    }

    #[inline]
    fn enter_state(&mut self, state: State) {
        info!("usb: bbb: Enter state: {}", state);
        // clean if going Idle
        if matches!(state, State::Idle) {
            self.buf.clean();
            self.cbw = Default::default();
            self.cs = None;
        }
        self.state.set(state);
    }
}

impl<'d, D, Buf> Transport for BulkOnly<'d, D, Buf>
where
    D: Driver<'d>,
    Buf: BorrowMut<[u8]>,
{
    const PROTO: u8 = TRANSPORT_BBB;
}

impl<'d> Handler for Control<'d> {
    fn reset(&mut self) {
        info!("usb: bbb: Recv reset");
        // self.in_ep.unstall();
        // self.out_ep.unstall();
        match self.state.get() {
            State::Idle => {}
            _ => {
                self.state.set(State::Reset);
            }
        }
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        // not interested in this request
        if !(req.request_type == RequestType::Class && req.recipient == Recipient::Interface) {
            return None;
        }

        info!("usb: bbb: Recv ctrl_in: {}", req);

        match req.request {
            // Spec. section 3.1
            CLASS_SPECIFIC_BULK_ONLY_MASS_STORAGE_RESET => None,
            // Spec. section 3.2
            CLASS_SPECIFIC_GET_MAX_LUN => {
                // always respond with LUN
                assert!(!buf.is_empty());
                buf[0] = self.max_lun;
                Some(InResponse::Accepted(&buf[0..1]))
            }
            _ => None,
        }
    }
}

#[derive(Default, Debug, Copy, Clone, Format)]
struct CommandBlockWrapper {
    tag: u32,
    data_transfer_len: u32,
    direction: DataDirection,
    lun: u8,
    block_len: usize,
    block: [u8; 16],
}

impl CommandBlockWrapper {
    fn from_le_bytes(value: &[u8]) -> Result<Self, InvalidCbwError> {
        const MIN_CB_LEN: u8 = 1;
        const MAX_CB_LEN: u8 = 16;

        let block_len = value[10];

        if !(MIN_CB_LEN..=MAX_CB_LEN).contains(&block_len) {
            return Err(InvalidCbwError);
        }

        Ok(CommandBlockWrapper {
            tag: u32::from_le_bytes(value[..4].try_into().unwrap()),
            data_transfer_len: u32::from_le_bytes(value[4..8].try_into().unwrap()),
            direction: if u32::from_le_bytes(value[4..8].try_into().unwrap()) != 0 {
                if (value[8] & (1 << 7)) > 0 {
                    DataDirection::In
                } else {
                    DataDirection::Out
                }
            } else {
                DataDirection::NotExpected
            },
            lun: value[9] & 0b00001111,
            block_len: block_len as usize,
            block: value[11..].try_into().unwrap(), // ok, cause we checked a length
        })
    }
}

struct OutEndPointReader<'a, 'd, D: Driver<'d>>(&'a mut D::EndpointOut);

impl<'a, 'd, D: Driver<'d>> AsyncWriter for OutEndPointReader<'a, 'd, D> {
    type Error = TransportError<BulkOnlyError>;

    async fn write(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        match self.0.read(buf).await {
            Ok(count) => Ok(count),
            Err(err) => Err(TransportError::Usb(err)),
        }
    }
}

struct InEndPointWriter<'a, 'd, D: Driver<'d>>(&'a mut D::EndpointIn);

impl<'a, 'd, D: Driver<'d>> AsyncReader for InEndPointWriter<'a, 'd, D> {
    type Error = TransportError<BulkOnlyError>;

    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        let packet_size = self.0.info().max_packet_size as usize;
        let len = min(packet_size, buf.len());
        if !buf.is_empty() {
            match self.0.write(&buf[..len]).await {
                Ok(_count) => Ok(len),
                Err(err) => Err(TransportError::Usb(err)),
            }
        } else {
            Ok(0) // not enough data in buf, though it's not an error
        }
    }
}
