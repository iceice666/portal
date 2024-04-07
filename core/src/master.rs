use std::{fs::File, io::Read, path::PathBuf, time::Duration};

use futures::{SinkExt, StreamExt};
use log::{debug, error, warn};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::{
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc::error::TryRecvError,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::instrument;

use crate::{
    codec::{MasterRequest, RequestCodec, ResponseCodec, SlaveResponse, MAX_FILE_FARGMENT_SIZE},
    error::{CrateResult, Error},
};

#[derive(Debug)]
pub enum TaskStatus {
    Running,
    Paused,
    Aborted,
}

/// Master
///
/// Send request to slaves
/// Receive response from slaves
#[derive(Debug)]
pub struct Master {
    sender: FramedWrite<OwnedWriteHalf, RequestCodec>,
    receiver: FramedRead<OwnedReadHalf, ResponseCodec>,
}

impl Master {
    /// Send a request.
    #[inline]
    async fn send_request(&mut self, request: MasterRequest) -> CrateResult {
        self.sender.feed(request).await?;
        self.sender.flush().await?;
        Ok(())
    }

    pub async fn close(&mut self) -> CrateResult {
        self.sender.close().await?;
        Ok(())
    }

    /// Keep receiving responses until the connection is closed.
    /// Using a channel to receive remote responses.
    pub(crate) async fn recv_responses(
        &mut self,
        msg_tx: UnboundedSender<SlaveResponse>,
    ) -> CrateResult {
        while let Some(resp) = self.receiver.next().await {
            let resp = resp?;
            debug!("Received response: {:#?}", resp);

            if msg_tx.send(resp).is_err() {
                debug!("Receiver dropped, quitting");
                break;
            }
        }
        Ok(())
    }
}

impl Master {
    /// Create a async task to send a file to the remote.
    ///
    /// # Argments
    /// * `path`: The path of the file to send.
    ///
    /// # Returns
    /// * A channel to control the task
    /// * A channel to receive the result
    pub async fn send_a_file(
        &mut self,
        path: PathBuf,
    ) -> (UnboundedSender<TaskStatus>, UnboundedReceiver<CrateResult>) {
        let (stat_sender, stat_receiver) = mpsc::unbounded_channel::<TaskStatus>();
        let (res_sender, res_receiver) = mpsc::unbounded_channel::<CrateResult>();

        let res = self.do_send_a_file(path, stat_receiver).await;

        if let Err(e) = res_sender.send(res) {
            error!("Failed to send the result: {:?}", e);
        }

        (stat_sender, res_receiver)
    }

    /// Send a file to the remote.
    ///
    /// # Argments
    /// * `path`: The path of the file to send.
    /// * `rx`: The receiver to receive the status of the task.
    ///         To control the task, send a [TaskStatus] with sender.
    ///
    /// # Returns
    /// * A result of this sending file task.
    #[instrument(skip(self, rx))]
    async fn do_send_a_file(
        &mut self,
        path: PathBuf,
        mut rx: UnboundedReceiver<TaskStatus>,
    ) -> CrateResult {
        let file_name = match path.file_name() {
            Some(v) => v.to_string_lossy().to_string(),
            None => "Untitled".to_string(),
        };

        let mut file = File::open(path.clone())?;
        let metadata = file.metadata()?;

        if !metadata.is_file() {
            return Err(Error::NotAFile);
        }

        // Send the file metadata
        let sha256 = match sha256::try_digest(path) {
            Ok(v) => v,
            Err(_) => return Err(Error::Sha256Digest),
        };

        // take first 2 chars as u8 as our file_id
        let file_id = u8::from_str_radix(&sha256[0..=1], 16).unwrap_or(255);

        // Tell the remote we are going to send a file
        let request = MasterRequest::FileMetadata {
            file_id,
            file_name,
            sha256,
        };
        debug!("Sending file metadata: {:#?}", request);
        self.send_request(request).await?;

        // Init
        let file_length = metadata.len();
        debug!("File length: {}", file_length);

        let mut buf = vec![0; MAX_FILE_FARGMENT_SIZE];
        let mut read_bytes = 0;
        let mut status = TaskStatus::Running;
        let mut fragment_index = 0;
        debug!("Prepare to send file");

        while read_bytes < file_length {
            // Update current status
            match rx.try_recv() {
                Ok(v) => status = v,
                Err(TryRecvError::Empty) => {}
                // Status channel is closed
                // We should drop the file and return
                Err(TryRecvError::Disconnected) => {
                    warn!("Status channel is closed, aborting. (file_id: {})", file_id);
                    let request = MasterRequest::DropFile { file_id };
                    self.send_request(request).await?;
                    return Ok(());
                }
            }

            debug!("Current status: {:#?}", status);

            // Determine the next action
            // If the status is Paused, we sleep 1 sec and continue to the next loop
            // If the status is Aborted, we drop the file and return
            // If the status is Running, we continue to the next step
            match status {
                TaskStatus::Running => {}
                TaskStatus::Paused => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                TaskStatus::Aborted => {
                    let request = MasterRequest::DropFile { file_id };
                    self.send_request(request).await?;
                    return Ok(());
                }
            }

            // Read the file content
            let read = file.read(&mut buf)? as u64;

            // If the read is 0, we reach the end of the file
            if read == 0 {
                break;
            }

            // Update counter
            read_bytes += read;

            debug!(
                "Read {} bytes from file. Total read {} bytes",
                read, read_bytes
            );

            // Build the request
            let request = MasterRequest::FileFragment {
                file_id,
                index: fragment_index,
                data: buf[0..read as usize].to_vec(),
            };
            // Send
            self.send_request(request).await?;

            fragment_index += 1;
            debug!("Next fragment index: {}", fragment_index);
        }

        // Send the end of file
        let request = MasterRequest::EndOfFile { file_id };
        self.send_request(request).await?;
        debug!("End of this file sending");

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn ping(&mut self) -> CrateResult {
        self.send_request(MasterRequest::Ping).await?;

        Ok(())
    }
}

impl From<tokio::net::TcpStream> for Master {
    fn from(value: TcpStream) -> Self {
        let (rd, wr) = value.into_split();

        let reader = FramedRead::new(rd, ResponseCodec);
        let writer = FramedWrite::new(wr, RequestCodec);

        Self {
            sender: writer,
            receiver: reader,
        }
    }
}

#[cfg(test)]
mod test {

    use std::net::TcpListener;

    use rand::Rng;

    use super::*;
    type AnyResult<T = ()> = anyhow::Result<T>;

    #[test]
    // cargo test max --lib -- --show-output
    fn test_max_cap() -> AnyResult<()> {
        let file = PathBuf::from("Cargo.toml");
        let file = File::open(file)?;

        let file_length = file.metadata()?.len();

        let mut rng = rand::thread_rng();

        let raw_data = rng
            .sample_iter(rand::distributions::Uniform::new(0, 255))
            .take(1498)
            .collect::<Vec<u8>>();

        let mut content_fargment_max_bytes = 0;
        while content_fargment_max_bytes < 1498 {
            content_fargment_max_bytes += 1;

            let mut buf = &raw_data[0..content_fargment_max_bytes];

            let request = MasterRequest::FileFragment {
                index: 0,
                data: buf.to_vec(),
                file_id: 0,
            };

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
