//! USB SCSI

use crate::usbd_storage::transport::bbb::StateHarder;
use crate::usbd_storage::{transport::Transport, CLASS_MASS_STORAGE};
use defmt::Format;
use embassy_usb::driver::Driver;
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;
use num_enum::TryFromPrimitive;
#[cfg(feature = "bbb")]
use {
    crate::usbd_storage::subclass::Command,
    crate::usbd_storage::transport::bbb::{BulkOnly, BulkOnlyError},
    crate::usbd_storage::transport::TransportError,
    core::borrow::BorrowMut,
};

/// SCSI device subclass code
pub const SUBCLASS_SCSI: u8 = 0x06; // SCSI Transparent command set

/* SCSI codes */

/* SPC */
const TEST_UNIT_READY: u8 = 0x00;
const REQUEST_SENSE: u8 = 0x03;
const INQUIRY: u8 = 0x12;
const MODE_SENSE_6: u8 = 0x1A;
const MODE_SENSE_10: u8 = 0x5A;

/* SBC */
const READ_10: u8 = 0x28;
const READ_16: u8 = 0x88;
const READ_CAPACITY_10: u8 = 0x25;
const READ_CAPACITY_16: u8 = 0x9E;
const WRITE_10: u8 = 0x2A;

/* MMC */
const READ_FORMAT_CAPACITIES: u8 = 0x23;

/// SCSI command
///
/// Refer to specifications (SPC,SAM,SBC,MMC,etc.)
#[derive(Copy, Clone, Debug, Format)]
pub enum ScsiCommand {
    Unknown,

    /* SPC */
    Inquiry {
        evpd: bool,
        page_code: u8,
        alloc_len: u16,
    },
    TestUnitReady,
    RequestSense {
        desc: bool,
        alloc_len: u8,
    },
    ModeSense6 {
        dbd: bool,
        page_control: PageControl,
        page_code: u8,
        subpage_code: u8,
        alloc_len: u8,
    },
    ModeSense10 {
        dbd: bool,
        page_control: PageControl,
        page_code: u8,
        subpage_code: u8,
        alloc_len: u16,
    },

    /* SBC */
    ReadCapacity10,
    ReadCapacity16 {
        alloc_len: u32,
    },
    Read {
        lba: u64,
        len: u64,
    },
    Write {
        lba: u64,
        len: u64,
    },

    /* MMC */
    ReadFormatCapacities {
        alloc_len: u16,
    },
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Format, TryFromPrimitive)]
pub enum PageControl {
    CurrentValues = 0b00,
    ChangeableValues = 0b01,
    DefaultValues = 0b10,
    SavedValues = 0b11,
}

#[allow(dead_code)]
fn parse_cb(cb: &[u8]) -> ScsiCommand {
    match cb[0] {
        TEST_UNIT_READY => ScsiCommand::TestUnitReady,
        INQUIRY => ScsiCommand::Inquiry {
            evpd: (cb[1] & 0b00000001) != 0,
            page_code: cb[2],
            alloc_len: u16::from_be_bytes([cb[3], cb[4]]),
        },
        REQUEST_SENSE => ScsiCommand::RequestSense {
            desc: (cb[1] & 0b00000001) != 0,
            alloc_len: cb[4],
        },
        READ_CAPACITY_10 => ScsiCommand::ReadCapacity10,
        READ_CAPACITY_16 => ScsiCommand::ReadCapacity16 {
            alloc_len: u32::from_be_bytes([cb[10], cb[11], cb[12], cb[13]]),
        },
        READ_10 => ScsiCommand::Read {
            lba: u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]) as u64,
            len: u16::from_be_bytes([cb[7], cb[8]]) as u64,
        },
        READ_16 => ScsiCommand::Read {
            lba: u64::from_be_bytes((&cb[2..10]).try_into().unwrap()),
            len: u32::from_be_bytes((&cb[10..14]).try_into().unwrap()) as u64,
        },
        WRITE_10 => ScsiCommand::Write {
            lba: u32::from_be_bytes([cb[2], cb[3], cb[4], cb[5]]) as u64,
            len: u16::from_be_bytes([cb[7], cb[8]]) as u64,
        },
        MODE_SENSE_6 => ScsiCommand::ModeSense6 {
            dbd: (cb[1] & 0b00001000) != 0,
            page_control: PageControl::try_from_primitive(cb[2] >> 6).unwrap(),
            page_code: cb[2] & 0b00111111,
            subpage_code: cb[3],
            alloc_len: cb[4],
        },
        MODE_SENSE_10 => ScsiCommand::ModeSense10 {
            dbd: (cb[1] & 0b00001000) != 0,
            page_control: PageControl::try_from_primitive(cb[2] >> 6).unwrap(),
            page_code: cb[2] & 0b00111111,
            subpage_code: cb[3],
            alloc_len: u16::from_be_bytes([cb[7], cb[8]]),
        },
        READ_FORMAT_CAPACITIES => ScsiCommand::ReadFormatCapacities {
            alloc_len: u16::from_be_bytes([cb[7], cb[8]]),
        },
        _ => ScsiCommand::Unknown,
    }
}

/// SCSI USB Mass Storage subclass
pub struct Scsi<T: Transport> {
    pub(super) transport: T,
}

/// SCSI subclass implementation with [Bulk Only Transport]
///
/// [Bulk Only Transport]: crate::transport::bbb::BulkOnly
#[cfg(feature = "bbb")]
impl<'d, D: Driver<'d>, Buf: BorrowMut<[u8]>> Scsi<BulkOnly<'d, D, Buf>> {
    /// Creates a SCSI over Bulk Only Transport instance
    ///
    /// # Arguments
    /// * `alloc` - [UsbBusAllocator]
    /// * `packet_size` - Maximum USB packet size. Allowed values: 8,16,32,64
    /// * `max_lun` - The max index of the Logical Unit
    /// * `buf` - The underlying IO buffer. It is **required** to fit at least a `CBW` and/or a single
    /// packet. It is **recommended** that buffer fits at least one sector
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
    /// [UsbBusAllocator]: usb_device::bus::UsbBusAllocator
    pub fn new<const CAP: usize>(
        builder: &mut Builder<'d, D>,
        state: &'d mut StateHarder<'d, CAP>,
        buf: Buf,
        packet_size: u16,
        max_lun: u8,
    ) -> Result<Self, BulkOnlyError> {
        let mut func = builder.function(
            CLASS_MASS_STORAGE,
            SUBCLASS_SCSI,
            <BulkOnly<D, Buf> as Transport>::PROTO,
        );
        let mut interface = func.interface();
        let mut alt = interface.alt_setting(
            CLASS_MASS_STORAGE,
            SUBCLASS_SCSI,
            <BulkOnly<D, Buf> as Transport>::PROTO,
            None,
        );
        let in_ep = alt.endpoint_bulk_in(packet_size);
        let out_ep = alt.endpoint_bulk_out(packet_size);
        drop(func);
        BulkOnly::new(builder, in_ep, out_ep, state, buf, max_lun)
            .map(|transport| Self { transport })
    }

    /// Drive subclass in both directions
    ///
    /// The passed closure may or may not be called after each time this function is called.
    /// Moreover, it may me called multiple times, if subclass is unable to proceed further.
    ///
    /// # Arguments
    /// * `callback` - closure, in which the SCSI command is processed
    pub async fn poll<F>(&mut self, mut callback: F) -> Result<(), EndpointError>
    where
        F: FnMut(Command<ScsiCommand, Scsi<BulkOnly<'d, D, Buf>>>),
    {
        fn map_ignore<T>(
            res: Result<T, TransportError<BulkOnlyError>>,
        ) -> Result<(), EndpointError> {
            match res {
                Ok(_) | Err(TransportError::Error(_)) => Ok(()),
                Err(TransportError::Usb(err)) => Err(err),
            }
        }
        // drive transport in both directions before user action
        map_ignore(self.transport.read().await)?;
        map_ignore(self.transport.write().await)?;

        if let Some(raw_cb) = self.transport.get_command() {
            // exec callback only if user action required
            if !self.transport.has_status() {
                let lun = raw_cb.lun;
                let kind = parse_cb(&raw_cb.bytes);

                loop {
                    callback(Command {
                        class: self,
                        kind,
                        lun,
                    });

                    // drive transport in both directions after user action.
                    // exec callback if not enough data
                    match self.transport.write().await {
                        Err(TransportError::Error(BulkOnlyError::FullPacketExpected)) => {
                            continue;
                        }
                        Ok(_) | Err(TransportError::Error(_)) => { /* ignore */ }
                        Err(TransportError::Usb(err)) => {
                            return Err(err);
                        }
                    };
                    map_ignore(self.transport.read().await)?;

                    break;
                }
            }
        }

        Ok(())
    }
}
