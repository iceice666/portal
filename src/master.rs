use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};

use crate::broadcast::Listener;

type AnyResult<T = ()> = anyhow::Result<T>;

enum DataFrameType {
    End,
    Data,
    Metadata,
}

impl From<DataFrameType> for u8 {
    fn from(value: DataFrameType) -> Self {
        match value {
            DataFrameType::End => 0,
            DataFrameType::Data => 1,
            DataFrameType::Metadata => 2,
        }
    }
}

/// MTU is 1500 bytes.
///
/// Frame format:
/// | Len | Position ID | Frame Type |    Data    |
/// |  2  |      2      |      1     |   n<=1495  |
#[derive(Debug)]
struct DataFrame {
    data: Vec<u8>,
    position_id: [u8; 2],
    frame_type: u8,
}

impl DataFrame {
    pub fn new(data: Vec<u8>, position_id: [u8; 2], frame_type: DataFrameType) -> AnyResult<Self> {
        if data.len() > 1495 {
            return Err(anyhow::anyhow!("Data too large"));
        }

        Ok(Self {
            data,
            position_id,
            frame_type: frame_type.into(),
        })
    }

    fn generate_u8_array(&self) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::new();

        let len = self.data.len() as u16;
        let len = [(len >> 8) as u8, len as u8];

        result.extend_from_slice(&len);
        result.extend_from_slice(&self.position_id);
        result.push(self.frame_type);
        result.extend(self.data.clone());
        result
    }
}

#[derive(Debug)]
pub struct Master {
    sender: OwnedReadHalf,
    receiver: OwnedWriteHalf,
}

impl Master {
    pub async fn new(listening_port: u16) -> AnyResult<Self> {
        let udp_listener = Listener::new(listening_port)?;
        let remote_service = udp_listener.recv_once()?;

        let tcp_stream = TcpStream::connect(remote_service).await?;
        let (sender, receiver) = tcp_stream.into_split();

        Ok(Self { sender, receiver })
    }
}
