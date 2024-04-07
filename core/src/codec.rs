use bytes::{BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio_util::codec;

use crate::error::{CrateResult, Error};
use crate::utils::u8_array_to_u16;

#[derive(Serialize, Deserialize)]
pub enum MasterRequest {
    // Ping!
    Ping,
    // file fragment data
    FileFragment {
        file_id: u8,
        // A 1 GiB has 1073741824 bytes
        // A fragment has 1477 bytes
        // 1073741824 / 1477 = 727051.5 -> 727052 < u32::MAX = 4_294_967_295
        //
        // And I think who will send a file that has 1 GiB with a small tool???
        index: u32,
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
pub enum SlaveResponse {
    // Pong!
    Pong,
    // Ok
    Ok,
    // File id is not found
    FileIdNotFound { file_id: u8 },
    // Cannot save the file
    CannotSaveFile { file_id: u8 },
    // Checksum is not matched
    ChecksumNotMatched { file_id: u8 },
}

impl Debug for MasterRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MasterRequest::Ping => write!(f, "Ping"),
            MasterRequest::FileFragment {
                file_id,
                index,
                data,
            } => {
                write!(
                    f,
                    "FileFragment: \nfile_id: {}, index: {}, data: (count: {})",
                    file_id,
                    index,
                    data.len()
                )
            }
            MasterRequest::FileMetadata {
                file_id,
                file_name,
                sha256,
            } => {
                write!(
                    f,
                    "FileMetadata: \nfile_id: {}, file_name: {}, sha256: {}",
                    file_id, file_name, sha256
                )
            }
            MasterRequest::EndOfFile { file_id } => {
                write!(f, "EndOfFile: file_id: {}", file_id)
            }
            MasterRequest::DropFile { file_id } => {
                write!(f, "DropFile: file_id: {}", file_id)
            }
        }
    }
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

impl codec::Encoder<SlaveResponse> for ResponseCodec {
    type Error = Error;

    fn encode(&mut self, item: SlaveResponse, dst: &mut BytesMut) -> CrateResult {
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

impl codec::Encoder<MasterRequest> for RequestCodec {
    type Error = Error;

    fn encode(&mut self, item: MasterRequest, dst: &mut BytesMut) -> CrateResult {
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
    type Item = SlaveResponse;

    fn decode(&mut self, src: &mut BytesMut) -> CrateResult<Option<SlaveResponse>> {
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
    type Item = MasterRequest;

    fn decode(&mut self, src: &mut BytesMut) -> CrateResult<Option<MasterRequest>> {
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

#[cfg(test)]
mod test {
    use std::{path::PathBuf, time::Duration};

    use crate::{master::Master, slave::Slave};
    use tokio::net::{TcpListener, TcpStream};
    use tracing_subscriber::filter::LevelFilter;

    #[tokio::test]
    async fn test() -> anyhow::Result<()> {
        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::DEBUG)
            .init();

        tokio::spawn(async {
            log::info!("Start the slave");
            let listener = TcpListener::bind("127.0.0.1:11451").await.unwrap();
            log::info!("Listening on: {}", listener.local_addr().unwrap());
            let (stream, _) = listener.accept().await.unwrap();
            let mut slave: Slave = stream.into();
            log::info!("Slave is ready");
            log::info!("Running recv loop");
            slave.recv_request_thread().await;
            log::info!("Slave is done");
        });

        tokio::time::sleep(Duration::from_secs(1)).await;

        log::info!("Start the master");
        let mut master: Master = TcpStream::connect("127.0.0.1:11451").await?.into();

        tokio::time::sleep(Duration::from_secs(1)).await;

        // log::info!("Send a ping");
        // master.ping().await?;
        // tokio::time::sleep(Duration::from_secs(1)).await;

        // log::info!("Send a file");
        // master.send_a_file(PathBuf::from("Cargo.toml")).await;

        // log::info!("Send a small file");
        // master.send_a_file(PathBuf::from("static/small.txt")).await;

        log::info!("Send a big file");
        master.send_a_file(PathBuf::from("static/big.txt")).await;

        tokio::time::sleep(Duration::from_secs(1)).await;
        log::info!("Close!");
        master.close().await?;

        Ok(())
    }
}
