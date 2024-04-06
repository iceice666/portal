use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use tokio_util::codec;

use crate::error::{CrateResult, Error};
use crate::utils::u8_array_to_u16;

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    // Ping!
    Ping,
    // plain data
    Data,
    // file fragment data
    FileFragment {
        file_id: u8,
        offset: u64,
        data: Vec<u8>,
    },
    // file metadata, and it represents the start of transfer a file
    FileMetadata {
        file_id: u8,
        file_name: String,
        sha256: String,
    },
    // end of file, and it represents the end of transfer a file
    EndOfFile {
        file_id: u8,
    },
    // drop a file, and it represents we abort the transfer of a file
    DropFile {
        file_id: u8,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Pong,
    Data,
    Ok,
}

#[derive(Debug)]
pub struct ResponseCodec;
#[derive(Debug)]
pub struct RequestCodec;

/// The MTU is 1500 bytes.
/// Frame format:
/// |        Max is 1500 bytes        |
/// |              | Warped by struct |
/// | Total Length |      Content     |
/// |    2 bytes   |    1498 bytes    |
const MAX_CONTENT_SIZE: usize = 1498;
/// Check [test_max_cap] test function
pub const MAX_FILE_FARGMENT_SIZE: usize = 1477;

impl codec::Encoder<Response> for ResponseCodec {
    type Error = Error;

    fn encode(&mut self, item: Response, dst: &mut BytesMut) -> CrateResult {
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

    fn encode(&mut self, item: Request, dst: &mut BytesMut) -> CrateResult {
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

    fn decode(&mut self, src: &mut BytesMut) -> CrateResult<Option<Response>> {
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

    fn decode(&mut self, src: &mut BytesMut) -> CrateResult<Option<Request>> {
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
