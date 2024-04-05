use std::{io, time::Duration};
use tracing::instrument;

use crate::broadcast;

pub struct SlaveConfig {
    udp_port: u16,
    service_port: u16,
}

pub struct Slave {
    config: SlaveConfig,
    broadcast_sender: broadcast::Sender,
}

impl Default for SlaveConfig {
    fn default() -> Self {
        Self {
            udp_port: 8964,
            service_port: 8965,
        }
    }
}

impl Slave {
    pub fn new(config: SlaveConfig) -> io::Result<Self> {
        let broadcast_sender = broadcast::Sender::new(config.service_port, config.udp_port)?;

        Ok(Self {
            config,
            broadcast_sender,
        })
    }

    #[instrument(skip(self))]
    pub async fn send_broadcast(&self) -> io::Result<()> {
        self.broadcast_sender.send_loop(Duration::from_secs(1))?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn receive_a_file(&self) -> io::Result<()> {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    type AnyResult<T = ()> = anyhow::Result<T>;
    use super::*;

    /// Test process
    /// 1. Send broadcast
    /// 2. Wait for connection
    /// 3. Receive a file
    /// 4. Receive the sha256 hash of the file
    /// 5. Compare the hash
    /// 6. Close the connection
    /// 7. End
    #[test]
    fn test() -> AnyResult {
        let slave = Slave::new(SlaveConfig::default())?;
        let handler = std::thread::spawn(|| {
            slave.broadcast_sender.send_loop(Duration::from_secs(1));
        });

        Ok(())
    }
}
