use std::{collections::HashSet, fs::File, io, net::SocketAddr, path::PathBuf, sync::Mutex};
use tracing::instrument;

use crate::{broadcast, side::Side};

pub struct MasterConfig {
    listening_port: u16,
}

pub struct Master {
    scanned_devices: Mutex<HashSet<SocketAddr>>,
    config: MasterConfig,
}

impl Default for MasterConfig {
    fn default() -> Self {
        Self {
            listening_port: 8964,
        }
    }
}

impl Master {
    pub fn new(config: MasterConfig) -> Self {
        Self {
            scanned_devices: Mutex::new(HashSet::new()),
            config,
        }
    }

    // main thread
    pub fn run(&self) {}

    #[instrument(skip(self))]
    pub fn scan_device(&mut self) -> io::Result<()> {
        let broadcast_listener = broadcast::Listener::new(self.config.listening_port)?;

        loop {
            let addr = broadcast_listener.recv_once()?;

            while let Ok(mut de) = self.scanned_devices.try_lock() {
                de.insert(addr);
            }
        }
    }

    #[instrument(skip(self))]
    async fn send_a_file(&self, addr: SocketAddr, file: PathBuf) -> io::Result<()> {
        let side = Side::new(addr).await?;
        let file = File::open(file)?;

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
