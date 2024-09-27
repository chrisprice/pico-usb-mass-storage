use core::fmt::Write;

use embassy_rp::i2c::{self, Blocking, Config, I2c};
use embassy_rp::peripherals::{I2C0, PIN_0, PIN_1};
use embassy_time::Timer;
use ssd1306::mode::{DisplayConfig, TerminalMode};
use ssd1306::prelude::{DisplayRotation, I2CInterface};
use ssd1306::{size::DisplaySize128x32, I2CDisplayInterface, Ssd1306};

use crate::SIGNAL;

pub struct Screen<'a> {
    address: [u8; 4],
    label: [u8; 11],
    freespace: u32,
    display: Ssd1306<I2CInterface<I2c<'a, I2C0, Blocking>>, DisplaySize128x32, TerminalMode>,
}

impl<'a> Screen<'a> {
    pub async fn build(i2c: I2C0, scl: PIN_1, sda: PIN_0) -> Self {
        let i2c = i2c::I2c::new_blocking(i2c, scl, sda, Config::default());

        let interface = I2CDisplayInterface::new(i2c);

        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_terminal_mode();
        display.init().unwrap();
        display.clear().unwrap();

        let _ = write!(display, "Hello, world");
        Self {
            address: [0; 4],
            label: [0; 11],
            freespace: 0,
            display,
        }
    }
    pub async fn run(&mut self) -> ! {
        Timer::after_secs(5).await;
        loop {
            match SIGNAL.wait().await {
                crate::DisplayState::Address(address) => {
                    self.address.copy_from_slice(&address);
                }
                crate::DisplayState::FileSystem(label, freespace) => {
                    self.label.copy_from_slice(&label);
                    self.freespace = freespace;
                }
            }
            self.display.clear().unwrap();
            let _ = writeln!(
                self.display,
                "{}.{}.{}.{}",
                self.address[0], self.address[1], self.address[2], self.address[3]
            );
            let _ = writeln!(self.display, "{}", unsafe {
                core::str::from_utf8_unchecked(&self.label)
            });
            //let _ = write!(self.display, "Free: ");
            super::human_bytes::write(&mut self.display, self.freespace);
            Timer::after_millis(100).await;
        }
    }
}
