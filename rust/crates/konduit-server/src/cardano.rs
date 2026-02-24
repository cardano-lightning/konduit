use cardano_connector_client::blockfrost;

mod args;
pub use args::CardanoArgs as Args;

// TODO: Not sure how hard it would be to turn CardanoConnector into a dyn compatible trait
// object. For now we use Blockfrost directly. In the future we can either share
// share the object or pass custom config of the connector via Data.
pub type Cardano = blockfrost::Connector;
