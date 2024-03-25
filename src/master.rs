use lazy_static::lazy_static;
use log::error;
use std::{collections::HashSet, fs::File, net::SocketAddr, path::PathBuf, sync::Mutex};
use tracing::instrument;

use crate::{broadcast, side::Side};

type AnyResult<T = ()> = anyhow::Result<T>;

lazy_static! {
    static ref LISTENING_PORT: u16 = std::env::var("BROADCAST_LISTENING_PORT")
        .unwrap_or_else(|_| "8964".to_string())
        .parse::<u16>()
        .expect("Invalid port number");
}

struct Master {
    scanned_devices: Mutex<HashSet<SocketAddr>>,
}

impl Master {
    pub fn new() -> Self {
        Self {
            scanned_devices: Mutex::new(HashSet::new()),
        }
    }

    // main thread
    pub fn run(&self) {}

    #[instrument(skip(self))]
    async fn broadcast_listening(&mut self) -> AnyResult {
        let broadcast_listener = broadcast::Listener::new(*LISTENING_PORT)?;

        loop {
            let addr = broadcast_listener.recv_once();

            if let Err(e) = addr {
                error!("Failed to receive broadcast: {}", e);
                continue;
            }

            let addr = addr.unwrap();

            while let Ok(mut de) = self.scanned_devices.try_lock() {
                de.insert(addr);
            }
        }
    }

    #[instrument(skip(self))]
    async fn send_a_file(&self, addr: SocketAddr, file: PathBuf) -> AnyResult {
        let side = Side::new(addr).await?;
        let file = File::open(file)?;

        Ok(())
    }
}
