use core::mem::MaybeUninit;

use defmt::info;
use defmt::Format;
use embassy_usb::control::InResponse;
use embassy_usb::control::Recipient;
use embassy_usb::control::Request;
use embassy_usb::control::RequestType;
use embassy_usb::driver::Driver;
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;

use crate::bulk_only_transport::CommandFailed;
use crate::scsi::Handler;
use crate::scsi::Scsi;

use self::endpoints::Endpoints;

pub mod endpoints;

const CLASS_MASS_STORAGE: u8 = 0x08;
const SUBCLASS_SCSI: u8 = 0x06; // SCSI Transparent command set
const PROTOCOL_BULK_ONLY_TRANSPORT: u8 = 0x50;

const CLASS_SPECIFIC_BULK_ONLY_MASS_STORAGE_RESET: u8 = 0xFF;
const CLASS_SPECIFIC_GET_MAX_LUN: u8 = 0xFE;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Format)]
pub enum Error {
    EndpointError(EndpointError),
}

// TODO: errors need revisiting
impl From<Error> for CommandFailed {
    fn from(err: Error) -> Self {
        match err {
            Error::EndpointError(e) => match e {
                EndpointError::BufferOverflow => CommandFailed,
                EndpointError::Disabled => CommandFailed,
            },
        }
    }
}

pub struct UsbMassStorage<'d, D: Driver<'d>> {
    scsi: Scsi<'d, D>,
}

impl<'d, D: Driver<'d>> UsbMassStorage<'d, D> {
    pub fn new(
        state: &'d mut State,
        builder: &mut Builder<'d, D>,
        packet_size: u16,
        max_lun: u8,
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
        );
        drop(func);

        let control = state.control.write(Control { max_lun });
        builder.handler(control);

        Self {
            scsi: Scsi::new(endpoints),
        }
    }

    pub async fn run(&mut self, handler: &mut impl Handler) {
        self.scsi.run(handler).await
    }
}

pub struct State {
    control: MaybeUninit<Control>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            control: MaybeUninit::uninit(),
        }
    }
}

pub struct Control {
    max_lun: u8,
}

impl embassy_usb::Handler for Control {
    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        // not interested in this request
        if !(req.request_type == RequestType::Class && req.recipient == Recipient::Interface) {
            return None;
        }

        info!("usb: bbb: Recv ctrl_in: {}", req);

        match req.request {
            // Spec. section 3.1
            // TODO: what would reset mean in this context?
            CLASS_SPECIFIC_BULK_ONLY_MASS_STORAGE_RESET => Some(InResponse::Rejected),
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
