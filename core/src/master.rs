use std::{collections::HashSet, fs::File, io, net::SocketAddr, path::PathBuf, sync::Mutex};
use tracing::instrument;

use crate::{broadcast, side::Side};

pub struct MasterConfig {
    udp_port: u16,
}

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
    pub fn new(config: MasterConfig) -> io::Result<Self> {
        let broadcast_listener = broadcast::Listener::new(config.udp_port)?;

        Ok(Self {
            scanned_devices: Mutex::new(HashSet::new()),
            config,
            broadcast_listener,
        })
    }

    #[instrument(skip(self))]
    pub fn scan_device(&mut self) -> io::Result<()> {
        loop {
            let addr = self.broadcast_listener.recv_once()?;

            while let Ok(mut de) = self.scanned_devices.try_lock() {
                de.insert(addr);
            }
        }
    }

    #[instrument(skip(self))]
    async fn send_a_file(&self, addr: SocketAddr, file: PathBuf) -> io::Result<()> {
        let side = Side::new(addr).await?;
        let file = File::open(file)?;

        unimplemented!()
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
    use super::*;

    /// Test process
    /// 1. Scan devices
    /// 2. Connect to the device
    /// 3. Send a file
    /// 4. Send the sha256 hash of the file
    /// 5. Close the connection
    /// 6. End
    #[test]
    fn test() -> AnyResult<()> {
        let master = Master::new(MasterConfig::default())?;

        unimplemented!()
    }
}
