use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use log::debug;
use serde::{Deserialize, Serialize};
use std::io;
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream, ToSocketAddrs,
};
use tokio::sync::mpsc;
use tokio_util::codec::{self, FramedRead, FramedWrite};

use crate::error::Error;
use crate::utils::u8_array_to_u16;

type AnyResult<T = ()> = anyhow::Result<T>;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum RequestKind {
    Ping,
    Data,
    FileFragment { offset: u64, data: Vec<u8> },
    FileMetadata { file_name: String, sha256: String },
    EndOfFile,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) enum ResponseKind {
    Pong,
    Data,
    Ok,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request(pub(crate) RequestKind);

#[derive(Debug, Serialize, Deserialize)]
pub struct Response(pub(crate) ResponseKind);

#[derive(Debug)]
pub struct ResponseCodec;
#[derive(Debug)]
pub struct RequestCodec;

/// The MTU is 1500 bytes.
/// Frame format:
/// |           Max is 1500 bytes           |
/// |              |    Warped by struct    |
/// | Total Length |  Kind  |    Content    |
/// |    2 bytes   | 1 byte |    n bytes    |
const MAX_CONTENT_SIZE: usize = 1498;

impl codec::Encoder<Response> for ResponseCodec {
    type Error = Error;

    fn encode(&mut self, item: Response, dst: &mut BytesMut) -> Result<(), Error> {
        let data = bincode::serialize(&item)?;
        let data_len = data.len();

        if data_len > MAX_CONTENT_SIZE {
            return Err(Error::DataTooLarge);
        }

        let data_len = 2 + data_len;

        dst.reserve(data_len);
        dst.put_u16(data_len as u16); // 2 bytes
        dst.extend_from_slice(data.as_slice()); // struct
        Ok(())
    }
}

impl codec::Encoder<Request> for RequestCodec {
    type Error = Error;

    fn encode(&mut self, item: Request, dst: &mut BytesMut) -> Result<(), Error> {
        let data = bincode::serialize(&item)?;
        let data_len = data.len();

        if data_len > MAX_CONTENT_SIZE {
            return Err(Error::DataTooLarge);
        }

        let data_len = 2 + data_len;

        dst.reserve(data_len);
        dst.put_u16(data_len as u16); // 2 bytes
        dst.extend_from_slice(data.as_slice()); // struct
        Ok(())
    }
}

impl codec::Decoder for ResponseCodec {
    type Error = Error;
    type Item = Response;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Response>, Self::Error> {
        if src.len() < 2 {
            return Ok(None);
        }

        let total_length = u8_array_to_u16([src[0], src[1]]) as usize;
        let buf_len = src.len();
        if buf_len < total_length {
            src.reserve(total_length - buf_len);
            return Ok(None);
        }

        let serialized = src.split_to(total_length);
        let response = bincode::deserialize(&serialized[2..])?;
        Ok(Some(response))
    }
}

impl codec::Decoder for RequestCodec {
    type Error = Error;
    type Item = Request;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Request>, Self::Error> {
        if src.len() < 2 {
            return Ok(None);
        }

        let total_length = u8_array_to_u16([src[0], src[1]]) as usize;
        let buf_len = src.len();
        if buf_len < total_length {
            src.reserve(total_length - buf_len);
            return Ok(None);
        }

        let serialized = src.split_to(total_length);
        let request = bincode::deserialize(&serialized[2..])?;
        Ok(Some(request))
    }
}

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
    pub async fn new(remote_service: impl ToSocketAddrs) -> io::Result<Self> {
        let tcp_stream = TcpStream::connect(remote_service).await?;
        let (rd, wr) = tcp_stream.into_split();
        let reader = FramedRead::new(rd, ResponseCodec);
        let writer = FramedWrite::new(wr, RequestCodec);

        Ok(Self { reader, writer })
    }

    /// Send a request.
    #[inline]
    pub async fn send_request(&mut self, request: Request) -> Result<(), Error> {
        self.writer.feed(request).await?;
        Ok(())
    }

    /// Keep receiving responses until the connection is closed.
    /// Using a channel to receive remote responses.
    pub async fn recv_responses(&mut self, msg_tx: mpsc::Sender<Response>) -> Result<(), Error> {
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
