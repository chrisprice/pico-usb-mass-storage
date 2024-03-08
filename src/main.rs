#![no_std]
#![no_main]

use core::mem::MaybeUninit;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    peripherals::USB,
    usb::{Driver, InterruptHandler},
};
use embassy_usb::{Builder, Config};
use panic_probe as _;
use pico_usb_mass_storage::usbd_storage::subclass::scsi::Scsi;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

/// Not necessarily `'static`. May reside in some special memory location
static mut USB_TRANSPORT_BUF: MaybeUninit<[u8; 512]> = MaybeUninit::uninit();
static mut STORAGE: [u8; (BLOCKS * BLOCK_SIZE) as usize] = [0u8; (BLOCK_SIZE * BLOCKS) as usize];

static mut STATE: State = State {
    storage_offset: 0,
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
    storage_offset: usize,
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

    // Required for windows compatibility.
    // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut [], // no msos descriptors
        &mut control_buf,
    );

    let mut scsi = pico_usb_mass_storage::usbd_storage::subclass::scsi::Scsi::new(
        &mut builder,
        USB_PACKET_SIZE,
        MAX_LUN,
        unsafe { USB_TRANSPORT_BUF.assume_init_mut().as_mut_slice() },
    )
    .unwrap();

    let mut usb = builder.build();
    let usb_fut = usb.run();

    // let echo_fut = async {
    //     loop {
    //         class.wait_connection().await;
    //         info!("Connected");
    //         let _ = echo(&mut class).await;
    //         info!("Disconnected");
    //     }
    // };

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    // join(usb_fut, echo_fut).await;

    usb_fut.await;

    // let mut usb_device = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0xabcd, 0xabcd))
    //     .manufacturer("Chris Price")
    //     .product("100k of your finest bytes")
    //     .serial_number("CP4096OYFB")
    //     .self_powered(false)
    //     .build();

    // loop {
    //     if !usb_device.poll(&mut [&mut scsi]) {
    //         continue;
    //     }

    //     // clear state if just configured or reset
    //     if matches!(usb_device.state(), UsbDeviceState::Default) {
    //         unsafe {
    //             STATE.reset();
    //         };
    //     }

    //     let _ = scsi.poll(|command| {
    //         if let Err(err) = process_command(command) {
    //             error!("{}", err);
    //         }
    //     });
    // }
}

// fn process_command(
//     mut command: Command<ScsiCommand, Scsi<BulkOnly<bsp::hal::usb::UsbBus, &mut [u8]>>>,
// ) -> Result<(), TransportError<BulkOnlyError>> {
//     match command.kind {
//         ScsiCommand::TestUnitReady { .. } => {
//             command.pass();
//         }
//         ScsiCommand::Inquiry { .. } => {
//             command.try_write_data_all(&[
//                 0x00, // periph qualifier, periph device type
//                 0x80, // Removable
//                 0x04, // SPC-2 compliance
//                 0x02, // NormACA, HiSu, Response data format
//                 0x20, // 36 bytes in total
//                 0x00, // additional fields, none set
//                 0x00, // additional fields, none set
//                 0x00, // additional fields, none set
//                 b'C', b'H', b'R', b'I', b'S', b'P', b' ', b' ', // 8-byte T-10 vendor id
//                 b'1', b'0', b'0', b'k', b' ', b'o', b'f', b' ', b'y', b'o', b'u', b'r', b' ', b'f',
//                 b'i', b'n', // 16-byte product identification
//                 b'1', b'.', b'2', b'3', // 4-byte product revision
//             ])?;
//             command.pass();
//         }
//         ScsiCommand::RequestSense { .. } => unsafe {
//             command.try_write_data_all(&[
//                 0x70,                         // RESPONSE CODE. Set to 70h for information on current errors
//                 0x00,                         // obsolete
//                 STATE.sense_key.unwrap_or(0), // Bits 3..0: SENSE KEY. Contains information describing the error.
//                 0x00,
//                 0x00,
//                 0x00,
//                 0x00, // INFORMATION. Device-specific or command-specific information.
//                 0x00, // ADDITIONAL SENSE LENGTH.
//                 0x00,
//                 0x00,
//                 0x00,
//                 0x00,                               // COMMAND-SPECIFIC INFORMATION
//                 STATE.sense_key_code.unwrap_or(0),  // ASC
//                 STATE.sense_qualifier.unwrap_or(0), // ASCQ
//                 0x00,
//                 0x00,
//                 0x00,
//                 0x00,
//             ])?;
//             STATE.reset();
//             command.pass();
//         },
//         ScsiCommand::ReadCapacity10 { .. } => {
//             let mut data = [0u8; 8];
//             let _ = &mut data[0..4].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
//             let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
//             command.try_write_data_all(&data)?;
//             command.pass();
//         }
//         ScsiCommand::ReadCapacity16 { .. } => {
//             let mut data = [0u8; 16];
//             let _ = &mut data[0..8].copy_from_slice(&u32::to_be_bytes(BLOCKS - 1));
//             let _ = &mut data[8..12].copy_from_slice(&u32::to_be_bytes(BLOCK_SIZE));
//             command.try_write_data_all(&data)?;
//             command.pass();
//         }
//         ScsiCommand::ReadFormatCapacities { .. } => {
//             let mut data = [0u8; 12];
//             let _ = &mut data[0..4].copy_from_slice(&[
//                 0x00, 0x00, 0x00, 0x08, // capacity list length
//             ]);
//             let _ = &mut data[4..8].copy_from_slice(&u32::to_be_bytes(BLOCKS as u32)); // number of blocks
//             data[8] = 0x01; //unformatted media
//             let block_length_be = u32::to_be_bytes(BLOCK_SIZE);
//             data[9] = block_length_be[1];
//             data[10] = block_length_be[2];
//             data[11] = block_length_be[3];

//             command.try_write_data_all(&data)?;
//             command.pass();
//         }
//         ScsiCommand::Read { lba, len } => unsafe {
//             let lba = lba as u32;
//             let len = len as u32;
//             if STATE.storage_offset != (len * BLOCK_SIZE) as usize {
//                 let start = (BLOCK_SIZE * lba) as usize + STATE.storage_offset;
//                 let end = (BLOCK_SIZE * lba) as usize + (BLOCK_SIZE * len) as usize;

//                 // Uncomment this in order to push data in chunks smaller than a USB packet.
//                 // let end = min(start + USB_PACKET_SIZE as usize - 1, end);

//                 let count = command.write_data(&mut STORAGE[start..end])?;
//                 STATE.storage_offset += count;
//             } else {
//                 command.pass();
//                 STATE.storage_offset = 0;
//             }
//         },
//         ScsiCommand::Write { lba, len } => unsafe {
//             let lba = lba as u32;
//             let len = len as u32;
//             if STATE.storage_offset != (len * BLOCK_SIZE) as usize {
//                 let start = (BLOCK_SIZE * lba) as usize + STATE.storage_offset;
//                 let end = (BLOCK_SIZE * lba) as usize + (BLOCK_SIZE * len) as usize;
//                 let count = command.read_data(&mut STORAGE[start..end])?;
//                 STATE.storage_offset += count;

//                 if STATE.storage_offset == (len * BLOCK_SIZE) as usize {
//                     command.pass();
//                     STATE.storage_offset = 0;
//                 }
//             } else {
//                 command.pass();
//                 STATE.storage_offset = 0;
//             }
//         },
//         ScsiCommand::ModeSense6 { .. } => {
//             command.try_write_data_all(&[
//                 0x03, // number of bytes that follow
//                 0x00, // the media type is SBC
//                 0x00, // not write-protected, no cache-control bytes support
//                 0x00, // no mode-parameter block descriptors
//             ])?;
//             command.pass();
//         }
//         ScsiCommand::ModeSense10 { .. } => {
//             command.try_write_data_all(&[0x00, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?;
//             command.pass();
//         }
//         ref unknown_scsi_kind => {
//             error!("Unknown SCSI command: {}", unknown_scsi_kind);
//             unsafe {
//                 STATE.sense_key.replace(0x05); // illegal request Sense Key
//                 STATE.sense_key_code.replace(0x20); // Invalid command operation ASC
//                 STATE.sense_qualifier.replace(0x00); // Invalid command operation ASCQ
//             }
//             command.fail();
//         }
//     }

//     Ok(())
// }
