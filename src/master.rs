use crate::frame::{Request, RequestCodec, ResponseCodec};
use futures::{SinkExt, StreamExt};
use log::debug;
use tokio::net::ToSocketAddrs;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};

type AnyResult<T = ()> = anyhow::Result<T>;

#[derive(Debug)]
pub struct Master {
    writer: FramedWrite<OwnedWriteHalf, RequestCodec>,
    reader: FramedRead<OwnedReadHalf, ResponseCodec>,
}

impl Master {
    pub async fn new(remote_service: impl ToSocketAddrs) -> AnyResult<Self> {
        let tcp_stream = TcpStream::connect(remote_service).await?;
        let (rd, wr) = tcp_stream.into_split();
        let reader = FramedRead::new(rd, ResponseCodec);
        let writer = FramedWrite::new(wr, RequestCodec);

        Ok(Self { reader, writer })
    }

    pub async fn send_request(&mut self, request: Request) -> AnyResult<()> {
        self.writer.send(request).await?;
        Ok(())
    }

    pub async fn recv_response(&mut self) -> AnyResult<()> {
        while let Some(resp) = self.reader.next().await {
            let resp = resp?;
            debug!("Received response: {:#?}", resp);

            // TODO: Use channel to send responses.
        }
        Ok(())
    }
}
