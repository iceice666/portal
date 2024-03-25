type AnyResult<T = ()> = anyhow::Result<T>;

use bytes::{BufMut, BytesMut};
use portal_macro::derive_conversion;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    io,
};
use tokio_util::codec::{self};

use crate::utils::u8_array_to_u16;

#[derive(Debug, thiserror::Error)]
pub enum ErrorKind {
    Io(io::Error),
    Bincode(bincode::Error),
    Disconnected,
    DataTooLarge,
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ErrorKind::Io(e) => write!(f, "IO error: {}", e),
            ErrorKind::Bincode(e) => write!(f, "Bincode error: {}", e),
            ErrorKind::Disconnected => write!(f, "Disconnected"),
            ErrorKind::DataTooLarge => write!(f, "Data too large"),
        }
    }
}

impl From<io::Error> for ErrorKind {
    fn from(e: io::Error) -> Self {
        ErrorKind::Io(e)
    }
}

derive_conversion!(
    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) enum RequestKind {
        Ping,
        Data,
        MetaData,
    }
);

derive_conversion!(
    #[derive(Debug, Serialize, Deserialize)]
    pub(crate) enum ResponseKind {
        Pong,
        Data,
        Ok,
    }
);

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    kind: RequestKind,
    content: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    kind: ResponseKind,
    content: Vec<u8>,
}

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
const MAX_CONTENT_LENGTH: usize = 1498;

impl codec::Encoder<Response> for ResponseCodec {
    type Error = ErrorKind;

    fn encode(&mut self, item: Response, dst: &mut BytesMut) -> Result<(), ErrorKind> {
        let data = bincode::serialize(&item).map_err(ErrorKind::Bincode)?;
        let data_len = data.len();

        if data_len > MAX_CONTENT_LENGTH {
            return Err(ErrorKind::DataTooLarge);
        }

        let data_len = 2 + data_len;

        dst.reserve(data_len);
        dst.put_u16(data_len as u16); // 2 bytes
        dst.extend_from_slice(data.as_slice()); // struct
        Ok(())
    }
}

impl codec::Encoder<Request> for RequestCodec {
    type Error = ErrorKind;

    fn encode(&mut self, item: Request, dst: &mut BytesMut) -> Result<(), ErrorKind> {
        let data = bincode::serialize(&item).map_err(ErrorKind::Bincode)?;
        let data_len = data.len();

        if data_len > MAX_CONTENT_LENGTH {
            return Err(ErrorKind::DataTooLarge);
        }

        let data_len = 2 + data_len;

        dst.reserve(data_len);
        dst.put_u16(data_len as u16); // 2 bytes
        dst.extend_from_slice(data.as_slice()); // struct
        Ok(())
    }
}

impl codec::Decoder for ResponseCodec {
    type Error = ErrorKind;
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
        let response = bincode::deserialize(&serialized[2..]).map_err(ErrorKind::Bincode)?;
        Ok(Some(response))
    }
}

impl codec::Decoder for RequestCodec {
    type Error = ErrorKind;
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
        let request = bincode::deserialize(&serialized[2..]).map_err(ErrorKind::Bincode)?;
        Ok(Some(request))
    }
}
