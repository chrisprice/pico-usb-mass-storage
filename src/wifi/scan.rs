use cyw43_pio::PioSpi;
use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::peripherals::{PIN_23, PIN_25};
use embassy_rp::{
    gpio::Output,
    peripherals::{DMA_CH0, PIO0},
};
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

pub struct Scan {}

impl Scan {
    pub async fn build(
        fw: &[u8],
        clm: &[u8],
        pwr: Output<'static, PIN_23>,
        spi: PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
        spawner: Spawner,
    ) -> Self {
        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (_net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
        unwrap!(spawner.spawn(wifi_task(runner)));

        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
            .await;

        let mut scanner = control.scan().await;
        while let Some(bss) = scanner.next().await {
            if let Ok(ssid_str) = core::str::from_utf8(&bss.ssid) {
                info!("scanned {} == {:x}", ssid_str, bss.bssid);
            }
        }
        Self {}
    }

    pub async fn run(&mut self) {
        // loop {
        //     info!("led on!");
        //     self.control.gpio_set(0, true).await;
        //     Timer::after(self.delay).await;

        //     info!("led off!");
        //     self.control.gpio_set(0, false).await;
        //     Timer::after(self.delay).await;
        // }
    }
}
