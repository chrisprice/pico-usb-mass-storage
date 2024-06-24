#![no_std]
#![no_main]

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_usb::{Builder, Config};
use embedded_io_async::ReadExactError;
use panic_probe as _;
use pico_usb_mass_storage::{bulk_only_transport::CommandError, usb_mass_storage::UsbMassStorage};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

static mut STORAGE: [u8; (BLOCKS * BLOCK_SIZE) as usize] = [0u8; (BLOCK_SIZE * BLOCKS) as usize];

static mut STATE: State = State {
    sense_key: None,
    sense_key_code: None,
    sense_qualifier: None,
};

const BLOCK_SIZE: u32 = 512;
const BLOCKS: u32 = 200;
const USB_PACKET_SIZE: u16 = 64; // 8,16,32,64
const MAX_LUN: u8 = 0; // max 0x0F

#[derive(Clone, Default)]
struct State {
    sense_key: Option<u8>,
    sense_key_code: Option<u8>,
    sense_qualifier: Option<u8>,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default();
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let driver = Driver::new(p.USB, Irqs);

    let mut config = Config::new(0xabcd, 0xabcd);
    config.manufacturer = Some("Chris Price");
    config.product = Some("100k of your finest bytes");
    config.serial_number = Some("CP4096OYFB");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut mos_descriptor = [0; 0];
    let mut control_buf = [0; 64];

    let mut usb_mass_storage_state = pico_usb_mass_storage::usb_mass_storage::State::default();
    let mut scsi_handler = Handler();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut mos_descriptor,
        &mut control_buf,
    );

    let mut usb_mass_storage: UsbMassStorage<'_, _, NoopRawMutex> = UsbMassStorage::new(
        &mut usb_mass_storage_state,
        &mut builder,
        USB_PACKET_SIZE,
        MAX_LUN,
    );

    let mut usb = builder.build();
    let usb_fut = usb.run();

    let usb_mass_storage_fut = usb_mass_storage.run(&mut scsi_handler);

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join(usb_fut, usb_mass_storage_fut).await;
}

struct Handler();

impl pico_usb_mass_storage::scsi::Handler for Handler {
    async fn read(
        &mut self,
        lba: u64,
        len: u64,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let lba = lba as u32;
        let len = len as u32;
        let start = (BLOCK_SIZE * lba) as usize;
        let end = start + (BLOCK_SIZE * len) as usize;
        for offset in (start..end).step_by(USB_PACKET_SIZE as usize) {
            writer
                .write_all(unsafe { &STORAGE[offset..offset + USB_PACKET_SIZE as usize] })
                .await?;
        }
        Ok(())
    }

    async fn write(
        &mut self,
        lba: u64,
        len: u64,
        reader: &mut impl embedded_io_async::Read<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        info!("write lba: {}, len: {}", lba, len);
        let lba = lba as u32;
        let len = len as u32;
        let start = (BLOCK_SIZE * lba) as usize;
        let end = start + (BLOCK_SIZE * len) as usize;
        for offset in (start..end).step_by(USB_PACKET_SIZE as usize) {
            reader
                .read_exact(unsafe { &mut STORAGE[offset..offset + USB_PACKET_SIZE as usize] })
                .await
                .map_err(|err| match err {
                    ReadExactError::UnexpectedEof => {
                        panic!("Unexpected EOF while writing to storage")
                    }
                    ReadExactError::Other(err) => err,
                })?;
        }
        Ok(())
    }

    async fn inquiry(
        &mut self,
        _evpd: bool,
        _page_code: u8,
        _alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [
            0x00, // periph qualifier, periph device type
            0x80, // Removable
            0x04, // SPC-2 compliance
            0x02, // NormACA, HiSu, Response data format
            0x20, // 36 bytes in total
            0x00, // additional fields, none set
            0x00, // additional fields, none set
            0x00, // additional fields, none set
            b'C', b'H', b'R', b'I', b'S', b'P', b' ', b' ', // 8-byte T-10 vendor id
            b'1', b'0', b'0', b'k', b' ', b'o', b'f', b' ', b'y', b'o', b'u', b'r', b' ', b'f',
            b'i', b'n', // 16-byte product identification
            b'1', b'.', b'2', b'3', // 4-byte product revision
        ];
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn test_unit_ready(&mut self) -> Result<(), CommandError> {
        Ok(())
    }

    async fn request_sense(
        &mut self,
        _desc: bool,
        _alloc_len: u8,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [
            0x70, // RESPONSE CODE. Set to 70h for information on current errors
            0x00, // obsolete
            unsafe { STATE.sense_key.unwrap_or(0) }, // Bits 3..0: SENSE KEY. Contains information describing the error.
            0x00,
            0x00,
            0x00,
            0x00, // INFORMATION. Device-specific or command-specific information.
            0x00, // ADDITIONAL SENSE LENGTH.
            0x00,
            0x00,
            0x00,
            0x00,                                          // COMMAND-SPECIFIC INFORMATION
            unsafe { STATE.sense_key_code.unwrap_or(0) },  // ASC
            unsafe { STATE.sense_qualifier.unwrap_or(0) }, // ASCQ
            0x00,
            0x00,
            0x00,
            0x00,
        ];
        writer.write_all(&data).await?;
        unsafe { STATE.reset() };
        Ok(())
    }

    async fn mode_sense6(
        &mut self,
        _dbd: bool,
        _page_control: pico_usb_mass_storage::scsi::command::PageControl,
        _page_code: u8,
        _subpage_code: u8,
        _alloc_len: u8,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [
            0x03, // number of bytes that follow
            0x00, // the media type is SBC
            0x00, // not write-protected, no cache-control bytes support
            0x00, // no mode-parameter block descriptors
        ];
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn mode_sense10(
        &mut self,
        _dbd: bool,
        _page_control: pico_usb_mass_storage::scsi::command::PageControl,
        _page_code: u8,
        _subpage_code: u8,
        _alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let data = [0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn read_capacity10(
        &mut self,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let mut data = [0u8; 8];
        let _ = &mut data[0..4].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
        let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn read_capacity16(
        &mut self,
        _alloc_len: u32,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let mut data = [0u8; 16];
        let _ = &mut data[0..8].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
        let _ = &mut data[8..12].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
        writer.write_all(&data).await?;
        Ok(())
    }

    async fn read_format_capacities(
        &mut self,
        _alloc_len: u16,
        writer: &mut impl embedded_io_async::Write<
            Error = pico_usb_mass_storage::usb_mass_storage::TransportError,
        >,
    ) -> Result<(), CommandError> {
        let mut data = [0u8; 12];
        let _ = &mut data[0..4].copy_from_slice(&[
            0x00, 0x00, 0x00, 0x08, // capacity list length
        ]);
        let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCKS)); // number of blocks
        data[8] = 0x01; //unformatted media
        let block_length_be = u32::to_be_bytes(BLOCK_SIZE);
        data[9] = block_length_be[1];
        data[10] = block_length_be[2];
        data[11] = block_length_be[3];

        writer.write_all(&data).await?;
        Ok(())
    }

    async fn unknown(&mut self) -> Result<(), CommandError> {
        error!("Unknown SCSI command");
        unsafe {
            STATE.sense_key.replace(0x05); // illegal request Sense Key
            STATE.sense_key_code.replace(0x20); // Invalid command operation ASC
            STATE.sense_qualifier.replace(0x00); // Invalid command operation ASCQ
        }
        Err(CommandError::CommandFailed)
    }
}
