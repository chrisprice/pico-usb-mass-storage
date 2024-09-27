use crate::server::SocketServer;
use crate::{DisplayState, SIGNAL};

use cyw43::Control;
use cyw43_pio::PioSpi;
use defmt::{info, unwrap, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::{tcp::TcpSocket, Config, Stack, StackResources};
use embassy_net_wiznet::Device;
use embassy_rp::peripherals::{PIN_23, PIN_25};
use embassy_rp::{
    gpio::Output,
    peripherals::{DMA_CH0, PIO0},
};
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

const WIFI_NETWORK: &str = env!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

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

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<cyw43::NetDriver<'static>>) -> ! {
    stack.run().await
}

pub struct Server<'a, S>
where
    S: SocketServer + Sized,
{
    control: Control<'static>,
    stack: &'a Stack<Device<'static>>,
    server: S,
}

impl<'a, S> Server<'a, S>
where
    S: SocketServer + Sized,
{
    pub async fn build(
        fw: &[u8],
        clm: &[u8],
        pwr: Output<'static, PIN_23>,
        spi: PioSpi<'static, PIN_25, PIO0, 0, DMA_CH0>,
        spawner: Spawner,
        server: S,
    ) -> Self {
        static STATE: StaticCell<cyw43::State> = StaticCell::new();
        let state = STATE.init(cyw43::State::new());
        let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
        unwrap!(spawner.spawn(wifi_task(runner)));

        control.init(clm).await;
        control
            .set_power_management(cyw43::PowerManagementMode::PowerSave)
            .await;

        let config = Config::dhcpv4(Default::default());
        //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
        //    dns_servers: Vec::new(),
        //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
        //});

        // Generate random seed
        let seed = 0x0123_4567_89ab_cdef; // chosen by fair dice roll. guarenteed to be random.

        // Init network stack
        static STACK: StaticCell<Stack<cyw43::NetDriver<'static>>> = StaticCell::new();
        static RESOURCES: StaticCell<StackResources<2>> = StaticCell::new();
        let stack = &*STACK.init(Stack::new(
            net_device,
            config,
            RESOURCES.init(StackResources::<2>::new()),
            seed,
        ));

        unwrap!(spawner.spawn(net_task(stack)));

        loop {
            //control.join_open(WIFI_NETWORK).await;
            match control.join_wpa2(WIFI_NETWORK, WIFI_PASSWORD).await {
                Ok(_) => break,
                Err(err) => {
                    info!("join failed with status={}", err.status);
                }
            }
        }

        // Wait for DHCP, not necessary when using static IP
        info!("waiting for DHCP...");
        while !stack.is_config_up() {
            Timer::after_millis(100).await;
        }
        info!("DHCP is now up!");

        let mut address: [u8; 4] = [0; 4];
        address.copy_from_slice(stack.config_v4().unwrap().address.address().as_bytes());

        SIGNAL.signal(DisplayState::Address(address));
        Self {
            control,
            stack,
            server,
        }
    }

    pub async fn run(&mut self) -> ! {
        let mut rx_buffer = [0; 4096];
        let mut tx_buffer = [0; 4096];
        //let mut buf = [0; 4096];

        loop {
            let mut socket = TcpSocket::new(self.stack, &mut rx_buffer, &mut tx_buffer);
            socket.set_timeout(Some(Duration::from_secs(10)));

            self.control.gpio_set(0, false).await;
            info!("Listening on TCP:1234...");
            if let Err(e) = socket.accept(1234).await {
                warn!("accept error: {:?}", e);
                continue;
            }

            info!("Received connection from {:?}", socket.remote_endpoint());
            self.control.gpio_set(0, true).await;

            self.server.run(socket).await;

            // loop {
            //     let n = match socket.read(&mut buf).await {
            //         Ok(0) => {
            //             warn!("read EOF");
            //             break;
            //         }
            //         Ok(n) => n,
            //         Err(e) => {
            //             warn!("read error: {:?}", e);
            //             break;
            //         }
            //     };

            //     info!("rxd {}", core::str::from_utf8(&buf[..n]).unwrap());

            //     match socket.write_all(&buf[..n]).await {
            //         Ok(()) => {}
            //         Err(e) => {
            //             warn!("write error: {:?}", e);
            //             break;
            //         }
            //     };
            // }
        }
    }
}
