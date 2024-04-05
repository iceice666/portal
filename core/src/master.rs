use std::{
    collections::HashSet,
    ffi::OsStr,
    fs::File,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tokio::net::TcpStream;
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
    use crate::side::Side;

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

        let tcp: Side = TcpStream::connect(device).await?.into();

        tcp.send_a_file(file);

        Ok(())
    }
}
