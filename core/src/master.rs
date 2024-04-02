use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::File,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tracing::instrument;

use crate::{
    broadcast,
    error::Error,
    side::{Request, RequestKind, Side},
};

type Result<T = ()> = std::result::Result<T, crate::error::Error>;

#[derive(Debug)]
pub struct MasterConfig {
    udp_port: u16,
}

#[derive(Debug)]
pub struct Master {
    scanned_devices: Mutex<HashSet<SocketAddr>>,
    config: MasterConfig,
    broadcast_listener: broadcast::Listener,
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self { udp_port: 8964 }
    }
}

impl Master {
    //! Why 1478? You can run the [test_max_cap] to find out.
    const CONTENT_FARGMENT_MAX_BYTES: usize = 1478;

    pub fn new(config: MasterConfig) -> Result<Self> {
        let broadcast_listener = broadcast::Listener::new(config.udp_port)?;

        Ok(Self {
            scanned_devices: Mutex::new(HashSet::new()),
            config,
            broadcast_listener,
        })
    }

    #[instrument(skip(self))]
    pub fn scan_device(&mut self) -> Result<()> {
        let addr = self.broadcast_listener.recv_once()?;

        while let Ok(mut de) = self.scanned_devices.try_lock() {
            de.insert(addr);
        }

        Ok(())
    }

    /// Send a file to the device
    #[instrument(skip(self))]
    async fn send_a_file(&self, addr: SocketAddr, path: PathBuf) -> Result<()> {
        let mut side = Side::new(addr).await?;

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

        side.send_request(request).await?;

        // Send the file content
        let file_length = metadata.len();

        let mut buf = vec![0; Self::CONTENT_FARGMENT_MAX_BYTES];
        let mut read_bytes = 0;
        let mut handler = file.take(Self::CONTENT_FARGMENT_MAX_BYTES as u64);

        while read_bytes < file_length {
            let read = handler.read(&mut buf)? as u64;
            read_bytes += read;

            let request = Request(RequestKind::FileFragment {
                offset: read_bytes,
                data: buf.clone(),
            });

            side.send_request(request).await?;
        }

        // Send the end of file
        let request = Request(RequestKind::EndOfFile);
        side.send_request(request).await?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_devices(&self) -> HashSet<SocketAddr> {
        loop {
            match self.scanned_devices.try_lock() {
                Ok(de) => return de.clone(),
                Err(_) => continue,
            }
        }
    }
}

#[cfg(test)]
mod test {
    type AnyResult<T = ()> = anyhow::Result<T>;
    use crate::utils::u64_to_u8_array;

    use super::*;

    /// Test process
    /// 1. Scan devices
    /// 2. Connect to the device
    /// 3. Send a file
    /// 4. Send the sha256 hash of the file
    /// 5. Close the connection
    /// 6. End
    #[tokio::test]
    async fn test() -> AnyResult<()> {
        let mut master = Master::new(MasterConfig::default())?;
        let file = PathBuf::from("test.txt");

        master.scan_device()?;
        let devices = master.get_devices().await;
        let device = devices.iter().next()?;

        master.send_a_file(device, file);

        Ok(())
    }

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
