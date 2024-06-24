#![no_std]
#![no_main]

use cyw43_pio::PioSpi;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    pio::Pio,
    usb::Driver,
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_usb::{Builder, Config};
use panic_probe as _;

mod scsi;
use scsi::{BlockDevice, BlockDeviceError};
mod usb_mass_storage;
use usb_mass_storage::UsbMassStorage;
mod bulk_only_transport;

mod fat12_partition;

#[cfg(feature = "wifi")]
mod wifi;
#[cfg(feature = "wifi")]
use wifi::{self, blinky::Blinky};

mod lib;

#[derive(Copy, Clone)]
struct Block([u8; BLOCK_SIZE]);
static mut STORAGE: [Block; BLOCKS as usize] = [Block::new(); BLOCKS as usize];

const BLOCK_SIZE: usize = 512;
const BLOCKS: u32 = 200;
const USB_PACKET_SIZE: u16 = 64; // 8,16,32,64
const MAX_LUN: u8 = 0; // max 0x0F

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    fat12_partition::init(unsafe { &mut STORAGE });

    let p = embassy_rp::init(Default::default());
    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, lib::Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    //let mut blinky = Blinky::build(fw, clm, pwr, spi, spawner).await;
    #[cfg(feature = "wifi")]
    let mut wifi = wifi::server::Server::build(fw, clm, pwr, spi, spawner).await;

    let driver = Driver::new(p.USB, lib::Irqs);

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

    let mut usb_mass_storage_state = usb_mass_storage::State::default();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut mos_descriptor,
        &mut control_buf,
    );

    let vendor_id = b"CHRISP  "; // per the spec, unused bytes should be a space
    let product_id = b"100k of trunc   ";
    let product_revision = b"1.24";

    let mut block_device = InMemoryBlockDevice;

    let mut usb_mass_storage = UsbMassStorage::<'_, '_, _, _, NoopRawMutex>::new(
        &mut usb_mass_storage_state,
        &mut builder,
        USB_PACKET_SIZE,
        MAX_LUN,
        &mut block_device,
        vendor_id,
        product_id,
        product_revision,
    );

    let mut usb = builder.build();
    let usb_fut = usb.run();

    let usb_mass_storage_fut = usb_mass_storage.run();

    #[cfg(feature = "wifi")]
    {
        let wifi_fut = wifi.run();
        embassy_futures::join::join3(
            usb_fut,
            usb_mass_storage_fut,
            wifi_fut
        ).await;
    }
    #[cfg(not(feature = "wifi"))]
    {
        embassy_futures::join::join(
            usb_fut,
            usb_mass_storage_fut
        ).await;
    }
}

struct InMemoryBlockDevice;

impl BlockDevice for InMemoryBlockDevice {
    const BLOCK_BYTES: usize = BLOCK_SIZE;

    // FIXME: reader/writer instead of buffers
    async fn read_block(&mut self, lba: u32, output: &mut [u8]) -> Result<(), BlockDeviceError> {
        assert_eq!(Self::BLOCK_BYTES, output.len());

        let block = unsafe { &STORAGE[lba as usize] };
        output.copy_from_slice(&block.0);

        Ok(())
    }

    async fn write_block(&mut self, lba: u32, input: &[u8]) -> Result<(), BlockDeviceError> {
        assert_eq!(Self::BLOCK_BYTES, input.len());

        let block = unsafe { &mut STORAGE[lba as usize] };
        block.0.copy_from_slice(input);

        unsafe {
            fat12_partition::log_fs(&mut STORAGE, BLOCKS as u64, BLOCK_SIZE as u64);
            for id in 0..4 {
                let partition = fat12_partition::read_partition(&STORAGE, id);
                info!("partition {}: {}", id, partition);
            }
        }

        Ok(())
    }

    fn block_count(&self) -> u32 {
        BLOCKS - 1
    }
}

impl Block {
    const fn new() -> Self {
        Self([0; BLOCK_SIZE])
    }
}
