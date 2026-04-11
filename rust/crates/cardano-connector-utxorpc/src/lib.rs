use cardano_sdk::Network;

/// Minimal task-001 placeholder surface.
///
/// The real UTxO RPC-backed `CardanoConnector` implementation is deferred to
/// task-002.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UtxoRpcPlaceholder {
    network: Network,
}

impl UtxoRpcPlaceholder {
    pub const fn new(network: Network) -> Self {
        Self { network }
    }

    pub const fn network(&self) -> Network {
        self.network
    }
}
