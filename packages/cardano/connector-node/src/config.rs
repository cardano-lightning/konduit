use std::path::PathBuf;

/// Configuration for a [`Node`](crate::Node) connector backed by a local Cardano
/// node reachable over the n2c Ouroboros miniprotocol suite.
#[derive(Clone, Debug)]
pub struct Config {
    socket: PathBuf,
    network_magic: u64,
}

impl Config {
    pub fn new(socket: impl Into<PathBuf>, network_magic: u64) -> Self {
        Self {
            socket: socket.into(),
            network_magic,
        }
    }

    pub fn socket(&self) -> &std::path::Path {
        &self.socket
    }

    pub fn network_magic(&self) -> u64 {
        self.network_magic
    }
}
