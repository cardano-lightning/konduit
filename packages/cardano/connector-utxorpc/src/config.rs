use cardano_sdk::PublicNetwork;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    endpoint: String,
    network: PublicNetwork,
}

impl Config {
    pub fn new(endpoint: impl Into<String>, network: PublicNetwork) -> Self {
        Self {
            endpoint: endpoint.into(),
            network,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub const fn network(&self) -> PublicNetwork {
        self.network
    }
}
