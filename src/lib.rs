#![no_std]

use embassy_rp::{
    bind_interrupts,
    peripherals::{PIO0, USB},
};

pub mod bulk_only_transport;
pub mod fat12_partition;
pub mod scsi;
pub mod usb_mass_storage;
pub mod wifi;

bind_interrupts!(pub struct Irqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});
