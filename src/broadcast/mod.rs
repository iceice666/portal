use std::{net::SocketAddr, time::Duration};

use log::info;
use tokio::net::UdpSocket;

type AnyResult<T = ()> = anyhow::Result<T>;

// magic 🤔
const MAGIC_THINKING_EMOJI: [u8; 36] = [
    49, 49, 52, 53, 49, 52, 32, 33, 112, 111, 114, 116, 97, 108, 32, 98, 114, 111, 97, 100, 99, 97,
    115, 116, 105, 110, 103, 33, 32, 49, 57, 49, 57, 56, 49, 48,
];

fn u16_to_u8_array(num: u16) -> [u8; 2] {
    let byte1 = (num >> 8) as u8;
    let byte2 = num as u8;
    [byte1, byte2]
}

fn u8_array_to_u16(arr: [u8; 2]) -> u16 {
    let byte1 = arr[0] as u16;
    let byte2 = arr[1] as u16;
    (byte1 << 8) | byte2
}

struct Sender {
    socket: UdpSocket,
    payload: Vec<u8>,
    port: u16,
}

impl Sender {
    pub async fn new(port: u16) -> AnyResult<Self> {
        let mut payload: Vec<u8> = u16_to_u8_array(port).to_vec();
        payload.extend_from_slice(&MAGIC_THINKING_EMOJI);

        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
        socket.set_broadcast(true)?;

        Ok(Self {
            socket,
            payload,
            port,
        })
    }

    pub async fn send_once(&self) -> AnyResult {
        self.socket
            .send_to(&self.payload, format!("255.255.255.255:{}", self.port))
            .await?;

        Ok(())
    }

    pub async fn send_infinite(&self, wait_duration: Duration) -> AnyResult {
        loop {
            self.send_once().await?;
            tokio::time::sleep(wait_duration).await;
        }
    }

    pub async fn send_n_times(&self, wait_duration: Duration, times: u64) -> AnyResult {
        for _ in 0..times {
            self.send_once().await?;
            tokio::time::sleep(wait_duration).await;
        }

        Ok(())
    }
}

struct Listener {
    socket: UdpSocket,
    port: u16,
}

impl Listener {
    pub async fn new(port: u16) -> AnyResult<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", port)).await?;
        socket.set_broadcast(true)?;

        Ok(Self { socket, port })
    }

    pub async fn listen_until_correct_message(&self) -> AnyResult<SocketAddr> {
        let mut buf = [0; 1024];
        loop {
            let (len, source) = self.socket.recv_from(&mut buf).await?;
            let port = u8_array_to_u16([buf[0], buf[1]]);
            let msg_byte = &buf[2..len];

            if msg_byte == MAGIC_THINKING_EMOJI {
                info!("Received correct message from {}:{}", source.ip(), port);

                return Ok(SocketAddr::new(source.ip(), port));
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    type AnyResult<T = ()> = anyhow::Result<T>;

    #[tokio::test]
    async fn test() -> AnyResult {
        let port = portpicker::pick_unused_port().expect("No free ports available");
        let sender = Sender::new(port).await?;
        sender.send_n_times(Duration::from_secs(1), 50).await?;

        Ok(())
    }
}

