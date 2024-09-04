#![no_std]

use embassy_rp::{
    bind_interrupts,
    peripherals::{PIO0, USB},
};

bind_interrupts!(pub struct Irqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<USB>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});
