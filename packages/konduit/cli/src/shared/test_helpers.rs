//! WARNING: Fully AI generated file.
//! Test-only helpers for constructing configs without touching a real chain.

use crate::config::{
    adaptor::Config as AdaptorConfig, admin::Config as AdminConfig, connector::Blockfrost,
    connector::Connector as ConnectorConfig, consumer::Config as ConsumerConfig,
};
use cardano_sdk::{Network, PublicNetwork, SigningKey};

const PREVIEW_SKEY: [u8; 32] = [9; 32];

pub fn admin() -> AdminConfig {
    let connector = ConnectorConfig::Blockfrost(Blockfrost {
        network: PublicNetwork::Preview,
        project_id: Some("preview1234".to_string()),
    });
    let wallet = SigningKey::from(PREVIEW_SKEY);
    let host_address = wallet
        .to_verification_key()
        .to_address(Network::PREVIEW.into());
    AdminConfig {
        connector,
        wallet,
        host_address,
    }
}

pub fn consumer() -> ConsumerConfig {
    let connector = ConnectorConfig::Blockfrost(Blockfrost {
        network: PublicNetwork::Preview,
        project_id: Some("preview1234".to_string()),
    });
    let wallet = SigningKey::from(PREVIEW_SKEY);
    let host_address = wallet
        .to_verification_key()
        .to_address(Network::PREVIEW.into());
    ConsumerConfig {
        connector,
        wallet,
        host_address,
    }
}

pub fn adaptor() -> AdaptorConfig {
    use konduit_data::Duration;
    let connector = ConnectorConfig::Blockfrost(Blockfrost {
        network: PublicNetwork::Preview,
        project_id: Some("preview1234".to_string()),
    });
    let wallet = SigningKey::from(PREVIEW_SKEY);
    let host_address = wallet
        .to_verification_key()
        .to_address(Network::PREVIEW.into());
    AdaptorConfig {
        connector,
        wallet,
        host_address,
        close_period: Duration::from_secs(60),
        fee: 0,
    }
}
