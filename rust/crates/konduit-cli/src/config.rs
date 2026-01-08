use cardano_tx_builder::{Address, Credential, Hash, NetworkId, address::kind};

pub mod admin;
pub mod connector;
pub mod consumer;
pub mod signing_key;
pub mod wallet;

pub fn placeholder_address() -> Address<kind::Shelley> {
    Address::new(
        NetworkId::MAINNET,
        Credential::from_key(Hash::<28>::new(vec![])),
    )
}
