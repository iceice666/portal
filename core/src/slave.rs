use std::{collections::HashMap, fs::File, io::Write};

use futures::{SinkExt, StreamExt};
use lazy_static::lazy_static;
use log::{debug, error, info};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::instrument;

use crate::{
    codec::{MasterRequest, RequestCodec, ResponseCodec, SlaveResponse},
    error::CrateResult,
};

/// Slave
///
/// Receive request from master
/// Send response to master
#[derive(Debug)]
pub struct Slave {
    sender: FramedWrite<OwnedWriteHalf, ResponseCodec>,
    receiver: FramedRead<OwnedReadHalf, RequestCodec>,
    // { file_id: [ (index, data), ...]}
    file_pool: HashMap<u8, Vec<(u32, Vec<u8>)>>,
    // { file_id: (file_name, sha256) }
    file_metadata: HashMap<u8, Metadata>,
}

#[derive(Debug)]
struct Metadata {
    file_name: String,
    sha256: String,
}

lazy_static! {
    static ref RECEVIED_FILE_FOLDER: String =
        std::env::var("RECEVIED_FILE_FOLDER").unwrap_or_else(|_| "received_files".to_string());
}

impl Slave {
    /// Send a response.
    #[inline]
    async fn send_response(&mut self, request: SlaveResponse) -> CrateResult {
        self.sender.feed(request).await?;
        self.sender.flush().await?;
        Ok(())
    }

    #[instrument(skip(self))]
    async fn handle_request(&mut self, request: MasterRequest) -> SlaveResponse {
        match request {
            MasterRequest::Ping => SlaveResponse::Pong,
            MasterRequest::FileFragment {
                file_id,
                index,
                data,
            } => {
                if let Some(file) = self.file_pool.get_mut(&file_id) {
                    file.push((index, data));
                    SlaveResponse::Ok
                } else {
                    SlaveResponse::FileIdNotFound { file_id }
                }
            }
            MasterRequest::FileMetadata {
                file_id,
                file_name,
                sha256,
            } => {
                self.file_metadata
                    .insert(file_id, Metadata { file_name, sha256 });
                self.file_pool.insert(file_id, Vec::new());
                SlaveResponse::Ok
            }

            MasterRequest::EndOfFile { file_id } => {
                // Remove the data from the pool
                let m = self.file_metadata.remove(&file_id);
                let n = self.file_pool.remove(&file_id);

                // If the file is not found, return error
                let Metadata { file_name, sha256 } = match m {
                    None => return SlaveResponse::FileIdNotFound { file_id },
                    Some(metadata) => metadata,
                };
                let mut file_content = match n {
                    None => return SlaveResponse::FileIdNotFound { file_id },
                    Some(file) => file,
                };

                #[cfg(not(debug_assertions))]
                let file_name = format!("{}/{}", RECEVIED_FILE_FOLDER.as_str(), file_name);

                #[cfg(debug_assertions)]
                let file_name = format!("static/recv_{}", file_name);

                // Sort the file content by index
                file_content.sort_by(|a, b| a.0.cmp(&b.0));

                // Combine the file content
                let file_content: Vec<u8> = file_content
                    .iter()
                    .flat_map(|(_, v)| v.iter())
                    .copied()
                    .collect();

                // Calculate the sha256
                let recv_sha256 = sha256::digest(file_content.as_slice());
                debug!("sha256: {}", recv_sha256);

                // Check the sha256
                if sha256 != recv_sha256 {
                    return SlaveResponse::ChecksumNotMatched { file_id };
                }

                // Create the file
                let mut file = match File::create(file_name.clone()) {
                    Err(_) => return SlaveResponse::CannotSaveFile { file_id },
                    Ok(file) => file,
                };

                // Write the file content
                if file.write_all(&file_content).is_err() {
                    return SlaveResponse::CannotSaveFile { file_id };
                }

                debug!("File saved: {}", file_name);

                SlaveResponse::Ok
            }
            MasterRequest::DropFile { file_id } => {
                self.file_pool.remove(&file_id);
                SlaveResponse::Ok
            }
        }
    }

    /// Keep receiving request until the connection is closed.
    #[instrument(skip(self))]
    pub async fn recv_request_thread(&mut self) -> CrateResult {
        debug!("Starting receiving request from master.");
        loop {
            match self.receiver.next().await {
                Some(Ok(request)) => {
                    debug!("Handle received request: {:#?}", request);
                    let slave_response = self.handle_request(request).await;

                    debug!("Send response: {:#?}", slave_response);
                    self.send_response(slave_response).await?;
                }

                Some(Err(e)) => {
                    error!("Failed to receive request: {:?}", e);
                    break Err(e);
                }

                None => {
                    info!("Connection closed.");
                    break Ok(());
                }
            }
        }
    }
}

impl From<tokio::net::TcpStream> for Slave {
    fn from(value: TcpStream) -> Self {
        let (rd, wr) = value.into_split();

        let reader = FramedRead::new(rd, RequestCodec);
        let writer = FramedWrite::new(wr, ResponseCodec);

        Self {
            sender: writer,
            receiver: reader,
            file_metadata: HashMap::new(),
            file_pool: HashMap::new(),
        }
    }
}
