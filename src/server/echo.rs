use defmt::{info, warn};
use embassy_net::tcp::TcpSocket;
use embedded_io_async::Write as _;

use super::SocketServer;

pub struct Server {}

impl Server {
    pub fn new() -> Self {
        Self {}
    }
}

impl SocketServer for Server {
    async fn run(&mut self, mut socket: TcpSocket<'_>) {
        let mut buf = [0; 4096];
        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("read error: {:?}", e);
                    break;
                }
            };

            info!("rxd {}", core::str::from_utf8(&buf[..n]).unwrap());

            match socket.write_all(&buf[..n]).await {
                Ok(()) => {}
                Err(e) => {
                    warn!("write error: {:?}", e);
                    break;
                }
            };
        }
    }
}
