use log::debug;
use std::collections::HashSet;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;
use tracing::instrument;

use crate::utils::{u16_to_u8_array, u8_array_to_u16};

const MAGIC: [u8; 7] = [0xB, 0x2D, 0xE, 0x13, 0x13, 0x8, 0xA];

#[derive(Debug)]
pub struct Sender {
    socket: UdpSocket,
    payload: Vec<u8>,
    broadcast_addr: SocketAddr,
}

impl Sender {
    pub fn new(service_port: u16, broadcast_port: u16) -> io::Result<Self> {
        let socket: UdpSocket = UdpSocket::bind("0.0.0.0:0")?;

        socket.set_broadcast(true)?;

        let mut payload: Vec<u8> = Vec::new();
        payload.extend_from_slice(&MAGIC);
        payload.extend(u16_to_u8_array(service_port));

        let broadcast_addr = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
            broadcast_port,
        );

        Ok(Self {
            socket,
            payload,
            broadcast_addr,
        })
    }

    #[instrument(skip(self))]
    pub fn send_loop(&self, period: Duration) -> io::Result<()> {
        loop {
            self.send_once()?;
            std::thread::sleep(period);
        }
    }

    #[instrument(skip(self))]
    pub async fn async_send_loop(&self, period: Duration) -> io::Result<()> {
        loop {
            self.send_once()?;
            tokio::time::sleep(period).await;
        }
    }

    #[instrument(skip(self))]
    pub fn send_once(&self) -> io::Result<()> {
        self.socket.send_to(&self.payload, self.broadcast_addr)?;
        debug!("Sent broadcast to {}", self.broadcast_addr);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Listener {
    socket: UdpSocket,
    pub scanned_devices: HashSet<SocketAddr>,
}

impl Listener {
    #[instrument]
    /// `listening_port` should be the same as the `broadcast_port` in the Sender.
    pub fn new(listening_port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", listening_port))?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(Duration::from_secs(30)))?;

        let scanned_devices = HashSet::new();

        debug!("Listening on port {}", listening_port);

        Ok(Self {
            socket,
            scanned_devices,
        })
    }

    #[instrument(skip(self))]
    pub(crate) fn recv_once(&self) -> io::Result<SocketAddr> {
        let mut buffer = [0; 1024];

        loop {
            let (_len, source) = self.socket.recv_from(&mut buffer).map_err(|e| {
                io::Error::new(e.kind(), format!("Failed to receive broadcast: {}", e))
            })?;
            debug!("Received broadcast from {}", source.ip(),);

            if buffer[0..7] == MAGIC {
                let port = u8_array_to_u16([buffer[7], buffer[8]]);
                debug!("Received correct message.");
                debug!("Remote service at {}:{}", source.ip(), port);

                return Ok(SocketAddr::new(source.ip(), port));
            }
        }
    }

    #[instrument(skip(self))]
    pub fn scan_device(&mut self) -> io::Result<()> {
        let addr = self.recv_once()?;

        self.scanned_devices.insert(addr);

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn async_scan_device(&'static mut self, duration: Duration) -> io::Result<()> {
        tokio::spawn(tokio::time::timeout(duration, async {
            loop {
                match self.recv_once() {
                    Err(_) => continue,
                    Ok(addr) => self.scanned_devices.insert(addr),
                };
            }
        }));
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{thread::sleep, time::Duration};

    use log::debug;
    use tracing::level_filters::LevelFilter;

    use super::{Listener, Sender};
    type AnyResult<T = ()> = anyhow::Result<T>;
    #[test]
    fn test() -> AnyResult {
        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::DEBUG)
            .init();

        let service_port = portpicker::pick_unused_port().expect("No ports available");
        let broadcast_port = portpicker::pick_unused_port().expect("No ports available");

        debug!(
            "service port: {}, broadcast port: {}",
            service_port, broadcast_port
        );

        let sender = Sender::new(service_port, broadcast_port)?;
        let listener = Listener::new(broadcast_port)?;

        std::thread::spawn(move || {
            debug!("Sending broadcast package...");
            sender.send_loop(std::time::Duration::from_secs(1))
        });

        sleep(Duration::from_secs(5));
        let addr = listener.recv_once()?;

        assert_eq!(addr.port(), service_port);

        Ok(())
    }
}
