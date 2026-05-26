use cardano_sdk::Network;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    endpoint: String,
    network: Network,
}

impl Config {
    pub fn new(endpoint: impl Into<String>, network: Network) -> Self {
        Self {
            endpoint: endpoint.into(),
            network,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub const fn network(&self) -> Network {
        self.network
    }
}
