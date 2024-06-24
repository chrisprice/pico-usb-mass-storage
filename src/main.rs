#![no_std]
#![no_main]

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
use panic_probe as _;

mod scsi;
use scsi::{BlockDevice, BlockDeviceError};
mod usb_mass_storage;
use usb_mass_storage::UsbMassStorage;
mod bulk_only_transport;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[derive(Copy, Clone)]
struct Block([u8; BLOCK_SIZE]);
static mut STORAGE: [Block; BLOCKS as usize] = [Block::new(); BLOCKS as usize];

const BLOCK_SIZE: usize = 512;
const BLOCKS: u32 = 200;
const USB_PACKET_SIZE: u16 = 64; // 8,16,32,64
const MAX_LUN: u8 = 0; // max 0x0F

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

    let vendor_id = b"CHRISP";
    let product_id = b"100k of trunc";
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

    join(usb_fut, usb_mass_storage_fut).await;
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
