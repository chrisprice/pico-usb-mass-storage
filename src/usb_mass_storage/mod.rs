use core::mem::MaybeUninit;

use defmt::info;
use defmt::Format;
use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::signal::Signal;
use embassy_usb::control::InResponse;
use embassy_usb::control::Recipient;
use embassy_usb::control::Request;
use embassy_usb::control::RequestType;
use embassy_usb::driver::Driver;
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;

use crate::bulk_only_transport::CommandError;
use crate::scsi::BlockDevice;
use crate::scsi::Scsi;

use self::endpoints::Endpoints;

pub mod endpoints;

const CLASS_MASS_STORAGE: u8 = 0x08;
const SUBCLASS_SCSI: u8 = 0x06; // SCSI Transparent command set
const PROTOCOL_BULK_ONLY_TRANSPORT: u8 = 0x50;

const CLASS_SPECIFIC_BULK_ONLY_MASS_STORAGE_RESET: u8 = 0xFF;
const CLASS_SPECIFIC_GET_MAX_LUN: u8 = 0xFE;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Format)]
pub enum TransportError {
    Endpoint(EndpointError),
    Reset(),
}

// TODO: errors need revisiting
impl From<TransportError> for CommandError {
    fn from(err: TransportError) -> Self {
        CommandError::TransportError(err)
    }
}

pub struct UsbMassStorage<'d, 'bd, D: Driver<'d>, BD: BlockDevice, M: RawMutex> {
    scsi: Scsi<'d, 'bd, D, BD, M>,
}

impl<'d, 'bd, D: Driver<'d>, BD: BlockDevice, M: RawMutex> UsbMassStorage<'d, 'bd, D, BD, M> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: &'d mut State<'d, M>,
        builder: &mut Builder<'d, D>,
        packet_size: u16,
        max_lun: u8,
        block_device: &'bd mut BD,
        vendor_identification: impl AsRef<[u8]>,
        product_identification: impl AsRef<[u8]>,
        product_revision_level: impl AsRef<[u8]>,
    ) -> Self {
        let mut func = builder.function(
            CLASS_MASS_STORAGE,
            SUBCLASS_SCSI,
            PROTOCOL_BULK_ONLY_TRANSPORT,
        );
        let mut interface = func.interface();
        let mut alt = interface.alt_setting(
            CLASS_MASS_STORAGE,
            SUBCLASS_SCSI,
            PROTOCOL_BULK_ONLY_TRANSPORT,
            None,
        );
        let endpoints = Endpoints::new(
            alt.endpoint_bulk_in(packet_size),
            alt.endpoint_bulk_out(packet_size),
            &state.reset_signal,
        );
        drop(func);

        let control = state.control.write(Control {
            reset_signal: &state.reset_signal,
            max_lun,
        });
        builder.handler(control);

        let scsi = Scsi::new(
            endpoints,
            block_device,
            vendor_identification,
            product_identification,
            product_revision_level,
            packet_size,
        );

        Self { scsi }
    }

    pub async fn run(&mut self) {
        self.scsi.run().await
    }
}

pub struct State<'d, M: RawMutex> {
    reset_signal: Signal<M, ()>,
    control: MaybeUninit<Control<'d, M>>,
}

impl<'d, M: RawMutex> Default for State<'d, M> {
    fn default() -> Self {
        Self {
            reset_signal: Signal::new(),
            control: MaybeUninit::uninit(),
        }
    }
}

pub struct Control<'d, M: RawMutex> {
    reset_signal: &'d Signal<M, ()>,
    max_lun: u8,
}

impl<'d, M: RawMutex> embassy_usb::Handler for Control<'d, M> {
    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        // not interested in this request
        if !(req.request_type == RequestType::Class && req.recipient == Recipient::Interface) {
            return None;
        }

        info!("usb: bbb: Recv ctrl_in: {}", req);

        match req.request {
            // Spec. section 3.1
            CLASS_SPECIFIC_BULK_ONLY_MASS_STORAGE_RESET => {
                self.reset_signal.signal(());
                Some(InResponse::Accepted(&[]))
            }
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
