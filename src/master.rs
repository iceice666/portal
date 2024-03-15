use tokio::io::ErrorKind::WouldBlock;
use tokio::net::ToSocketAddrs;

use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

use crate::frame::DataFrame;

type AnyResult<T = ()> = anyhow::Result<T>;

#[derive(Debug)]
pub struct Master {
    receiver: OwnedReadHalf,
    sender: OwnedWriteHalf,
}

impl Master {
    pub async fn new(remote_service: impl ToSocketAddrs) -> AnyResult<Self> {
        let tcp_stream = TcpStream::connect(remote_service).await?;
        let (receiver, sender) = tcp_stream.into_split();

        Ok(Self { sender, receiver })
    }

    async fn send(&mut self, data: DataFrame) -> AnyResult {
        let data: Vec<u8> = data.into();
        loop {
            self.sender.writable().await?;
            match self.sender.try_write(&data) {
                Ok(_) => break,
                Err(ref e) if e.kind() == WouldBlock => continue,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }
}
