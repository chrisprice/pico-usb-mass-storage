use cyw43::Control;
use cyw43_pio::PioSpi;
use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::pio::Pio;
use embassy_rp::{
    gpio::{Level, Output},
    peripherals::{DMA_CH0, PIO0},
};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

#[embassy_executor::task]
async fn wifi_task(
    runner: cyw43::Runner<
        'static,
        Output<'static, embassy_rp::peripherals::PIN_23>,
        PioSpi<'static, embassy_rp::peripherals::PIN_25, PIO0, 0, DMA_CH0>,
    >,
) -> ! {
    runner.run().await
}

pub struct Blinky {
    delay: Duration,
    control: Control<'static>,
}
impl Blinky {
    pub async fn build(
        pin23: embassy_rp::peripherals::PIN_23,
        pin24: embassy_rp::peripherals::PIN_24,
        pin25: embassy_rp::peripherals::PIN_25,
        pin29: embassy_rp::peripherals::PIN_29,
        pio0: embassy_rp::peripherals::PIO0,
        dma_ch0: embassy_rp::peripherals::DMA_CH0,
        spawner: Spawner,
    ) -> Self {
        let fw = include_bytes!("../../cyw43-firmware/43439A0.bin");
        let clm = include_bytes!("../../cyw43-firmware/43439A0_clm.bin");

        let pwr = Output::new(pin23, Level::Low);
        let cs = Output::new(pin25, Level::High);
        let mut pio = Pio::new(pio0, super::Irqs);
        let spi = PioSpi::new(
            &mut pio.common,
            pio.sm0,
            pio.irq0,
            cs,
            pin24,
            pin29,
            dma_ch0,
        );

        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
        unwrap!(spawner.spawn(wifi_task(runner)));

        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
            .await;
        let delay = Duration::from_secs(1);
        Self { delay, control }
    }

    pub async fn run(&mut self) -> ! {
        loop {
            info!("led on!");
            self.control.gpio_set(0, true).await;
            Timer::after(self.delay).await;

            info!("led off!");
            self.control.gpio_set(0, false).await;
            Timer::after(self.delay).await;
        }
    }
}
