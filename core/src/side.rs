use std::{fs::File, io::Read, path::PathBuf};

use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use log::debug;
use serde::{Deserialize, Serialize};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio::sync::mpsc;
use tokio_util::codec::{self, FramedRead, FramedWrite};
use tracing::instrument;

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

type Result<T = ()> = std::result::Result<T, Error>;

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

    fn encode(&mut self, item: Response, dst: &mut BytesMut) -> Result {
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

    fn encode(&mut self, item: Request, dst: &mut BytesMut) -> Result {
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

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Response>> {
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

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Request>> {
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

/// Why 1478? You can run the [test_max_cap] to find out.
const CONTENT_FARGMENT_MAX_BYTES: usize = 1478;

impl Side {
    /// Send a request.
    #[inline]
    pub async fn send_request(&mut self, request: Request) -> Result {
        self.writer.feed(request).await?;
        Ok(())
    }

    /// Keep receiving responses until the connection is closed.
    /// Using a channel to receive remote responses.
    pub async fn recv_responses(&mut self, msg_tx: mpsc::Sender<Response>) -> Result {
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

    #[instrument(skip(self))]
    pub async fn send_a_file(&mut self, path: PathBuf) -> Result {
        let file_name = match path.file_name() {
            Some(v) => v.to_string_lossy().to_string(),
            None => "Untitled".to_string(),
        };

        let file = File::open(path.clone())?;
        let metadata = file.metadata()?;

        if !metadata.is_file() {
            return Err(Error::NotAFile);
        }

        // Send the file metadata
        let sha256 = match sha256::try_digest(path) {
            Ok(v) => v,
            Err(_) => return Err(Error::Sha256Digest),
        };

        let request = Request(RequestKind::FileMetadata { file_name, sha256 });

        self.send_request(request).await?;

        // Send the file content
        let file_length = metadata.len();

        let mut buf = vec![0; CONTENT_FARGMENT_MAX_BYTES];
        let mut read_bytes = 0;
        let mut handler = file.take(CONTENT_FARGMENT_MAX_BYTES as u64);

        while read_bytes < file_length {
            let read = handler.read(&mut buf)? as u64;
            read_bytes += read;

            let request = Request(RequestKind::FileFragment {
                offset: read_bytes,
                data: buf.clone(),
            });

            self.send_request(request).await?;
        }

        // Send the end of file
        let request = Request(RequestKind::EndOfFile);
        self.send_request(request).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn ping(&mut self) -> Result {
        self.send_request(Request(RequestKind::Ping)).await?;

        Ok(())
    }
}

impl From<TcpStream> for Side {
    fn from(value: TcpStream) -> Self {
        let (rd, wr) = value.into_split();

        let reader = FramedRead::new(rd, ResponseCodec);
        let writer = FramedWrite::new(wr, RequestCodec);

        Self { writer, reader }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    // cargo test max --lib -- --show-output
    fn test_max_cap() -> AnyResult<()> {
        let file = PathBuf::from("Cargo.toml");
        let file = File::open(file)?;

        let file_length = file.metadata()?.len();

        let mut content_fargment_max_bytes = 0;
        while content_fargment_max_bytes < 1498 {
            content_fargment_max_bytes += 1;

            let mut buf = vec![0; content_fargment_max_bytes];

            let request = Request(RequestKind::FileFragment {
                offset: 114514,
                data: buf.clone(),
            });

            let data = bincode::serialize(&request)?;
            let data_len = data.len();

            if data_len > 1498 {
                println!("Max cap is {}", content_fargment_max_bytes - 1);
                break;
            }
        }

        Ok(())
    }
}
