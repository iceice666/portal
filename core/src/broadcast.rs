use log::debug;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;
use tracing::instrument;

use crate::utils::{u16_to_u8_array, u8_array_to_u16};

type AnyResult<T = ()> = anyhow::Result<T>;

const MAGIC: [u8; 7] = [0xb, 0x2d, 0xe, 0x13, 0x13, 0x8, 0xa];

pub struct Sender {
    socket: UdpSocket,
    payload: Vec<u8>,
    broadcast_addr: SocketAddr,
}

impl Sender {
    pub fn new(service_port: u16, broadcast_port: u16) -> AnyResult<Self> {
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

    pub fn send_loop(&self, period: Duration) -> AnyResult<()> {
        loop {
            self.send_once()?;
            std::thread::sleep(period);
        }
    }

    #[instrument(skip(self))]
    pub fn send_once(&self) -> AnyResult<()> {
        self.socket.send_to(&self.payload, self.broadcast_addr)?;
        debug!("Sent broadcast to {}", self.broadcast_addr);

        Ok(())
    }
}

#[derive(Debug)]
pub struct Listener {
    socket: UdpSocket,
}

impl Listener {
    #[instrument]
    /// listening_port should be the same as the broadcast_port in the Sender.
    pub fn new(listening_port: u16) -> AnyResult<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", listening_port))?;
        socket.set_broadcast(true)?;

        debug!("Listening on port {}", listening_port);

        Ok(Self { socket })
    }

    #[instrument(skip(self))]
    pub fn recv_once(&self) -> AnyResult<SocketAddr> {
        let mut buffer = [0; 1024];

        loop {
            let (len, source) = self.socket.recv_from(&mut buffer)?;
            debug!("Received broadcast from {}", source.ip(),);

            if buffer[0..7] == MAGIC {
                let port = u8_array_to_u16([buffer[7], buffer[8]]);
                debug!("Received correct message.");
                debug!("Remote service at {}:{}", source.ip(), port);

                return Ok(SocketAddr::new(source.ip(), port));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{thread::sleep, time::Duration};

    use super::{Listener, Sender};
    type AnyResult<T = ()> = anyhow::Result<T>;
    #[test]
    fn test() -> AnyResult {
        tracing_subscriber::fmt::init();

        let service_port = portpicker::pick_unused_port().expect("No ports available");
        let broadcast_port = portpicker::pick_unused_port().expect("No ports available");

        let sender = Sender::new(service_port, broadcast_port)?;
        let listener = Listener::new(broadcast_port)?;

        let sender_handle =
            std::thread::spawn(move || sender.send_loop(std::time::Duration::from_secs(1)));

        sleep(Duration::from_secs(5));
        let addr = listener.recv_once()?;
        assert_eq!(addr.port(), service_port);

        Ok(())
    }
}