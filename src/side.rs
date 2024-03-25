use crate::frame::{Request, RequestCodec, Response, ResponseCodec};
use futures::{SinkExt, StreamExt};
use log::debug;
use tokio::net::ToSocketAddrs;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite};

type AnyResult<T = ()> = anyhow::Result<T>;

/// A side
/// Send a request with [send_request]
/// Request => [send_request] => remote
///
/// Pass a channel to [recv_responses] to get responses
/// remote => [recv_responses] -> channel sender => another thread/process
#[derive(Debug)]
pub struct Side {
    writer: FramedWrite<OwnedWriteHalf, RequestCodec>,
    reader: FramedRead<OwnedReadHalf, ResponseCodec>,
}

impl Side {
    pub async fn new(remote_service: impl ToSocketAddrs) -> AnyResult<Self> {
        let tcp_stream = TcpStream::connect(remote_service).await?;
        let (rd, wr) = tcp_stream.into_split();
        let reader = FramedRead::new(rd, ResponseCodec);
        let writer = FramedWrite::new(wr, RequestCodec);

        Ok(Self { reader, writer })
    }

    /// Send a request.
    #[inline]
    pub async fn send_request(&mut self, request: Request) -> AnyResult<()> {
        self.writer.send(request).await?;
        Ok(())
    }

    /// Keep receiving responses until the connection is closed.
    /// Using a channel to receive remote responses.
    pub async fn recv_responses(&mut self, msg_tx: mpsc::Sender<Response>) -> AnyResult<()> {
        while let Some(resp) = self.reader.next().await {
            let resp = resp?;
            debug!("Received response: {:#?}", resp);

            if msg_tx.send(resp).await.is_err() {
                debug!("Receiver dropped, quitting");
                break;
            }
        }
        Ok(())
    }
}
