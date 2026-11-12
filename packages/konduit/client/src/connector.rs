//! Backend selection for the client's L1 arm.
//!
//! The same three knobs konduit-cli reads (`KONDUIT_CARDANO_BACKEND`,
//! `KONDUIT_NETWORK`, `KONDUIT_NODE_SOCKET`/`KONDUIT_KUPO_URI`), so a client
//! invocation and a cli invocation in the same shell talk to the same chain.
//!
//! Only blockfrost is always available: the node-backed backends are behind
//! the `node` feature, since their transports don't build for wasm.

use anyhow::anyhow;
use cardano_sdk::Network;
use std::fmt;

#[cfg(feature = "node")]
use anyhow::Context;
#[cfg(feature = "node")]
use cardano_connector::CardanoConnector;
#[cfg(feature = "node")]
use cardano_connector_node::{Config as NodeConfig, Node};
#[cfg(feature = "node")]
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Backend {
    Blockfrost,
    #[cfg(feature = "node")]
    Node,
    #[cfg(feature = "node")]
    KupoNode,
}

impl fmt::Display for Backend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Blockfrost => "blockfrost",
            #[cfg(feature = "node")]
            Self::Node => "node",
            #[cfg(feature = "node")]
            Self::KupoNode => "kupo-node",
        })
    }
}

/// Which chain backend the L1 commands talk to, and what each one needs.
#[derive(Debug, clap::Args)]
pub struct ConnectorArgs {
    #[arg(long = "backend", env = "KONDUIT_CARDANO_BACKEND", default_value_t = Backend::Blockfrost)]
    pub backend: Backend,

    /// Required by the node-backed backends: the network magic the node runs.
    #[arg(long, env = "KONDUIT_NETWORK")]
    pub network: Option<Network>,

    /// Required by the blockfrost backend.
    #[arg(long, env = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost_project_id: Option<String>,

    /// Required by the node-backed backends.
    #[arg(long, env = "KONDUIT_NODE_SOCKET")]
    pub node_socket: Option<String>,

    /// Used by the kupo-node backend.
    #[arg(
        long,
        env = "KONDUIT_KUPO_URI",
        default_value = "http://127.0.0.1:1443"
    )]
    pub kupo_uri: String,
}

impl ConnectorArgs {
    pub fn blockfrost_project_id(&self) -> anyhow::Result<&str> {
        self.blockfrost_project_id.as_deref().ok_or_else(|| {
            anyhow!("Cardano backend blockfrost requires KONDUIT_BLOCKFROST_PROJECT_ID")
        })
    }

    /// The node config both node-backed backends dial, checked for the bits
    /// neither can be inferred.
    #[cfg(feature = "node")]
    fn node_config(&self) -> anyhow::Result<NodeConfig> {
        let network = self
            .network
            .ok_or_else(|| anyhow!("Cardano backend {} requires KONDUIT_NETWORK", self.backend))?;
        let socket = self.node_socket.as_deref().ok_or_else(|| {
            anyhow!(
                "Cardano backend {} requires KONDUIT_NODE_SOCKET",
                self.backend
            )
        })?;

        Ok(NodeConfig::new(PathBuf::from(socket), network.magic()))
    }

    /// Connects, then health-checks: a dead socket should fail here, naming
    /// the socket, rather than inside whatever L1 query runs first.
    #[cfg(feature = "node")]
    pub async fn node(&self) -> anyhow::Result<Node> {
        let config = self.node_config()?;
        let socket = config.socket().display().to_string();
        let magic = config.network_magic();

        let connector = Node::connect(config).await.with_context(|| {
            format!("failed to connect to local Cardano node at {socket} (network magic {magic})")
        })?;
        connector
            .health()
            .await
            .with_context(|| format!("failed to reach local Cardano node at {socket}"))?;

        Ok(connector)
    }

    #[cfg(feature = "node")]
    pub async fn kupo_node(&self) -> anyhow::Result<cardano_connector_kupo_node::KupoNode> {
        let config = self.node_config()?;

        cardano_connector_kupo_node::KupoNode::connect(&self.kupo_uri, config)
            .await
            .with_context(|| format!("failed to connect Kupo + node backend at {}", self.kupo_uri))
    }
}
