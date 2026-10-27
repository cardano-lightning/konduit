use anyhow::Result;
use cardano_connector::CardanoConnector;
use cardano_connector_node::{Config, Node};
use cardano_sdk::{Address, address::kind};

#[tokio::main]
async fn main() -> Result<()> {
    let socket = std::env::var("CARDANO_NODE_SOCKET_PATH").unwrap_or_else(|_| {
        "/home/paluh/projects/cl/konduit/paluh/indexer/.run/testnet/state-cluster9/bft1.socket"
            .to_string()
    });
    let network_id: u64 = std::env::var("CARDANO_NODE_NETWORK_ID")
        .unwrap_or_else(|_| "42".to_string())
        .parse()?;

    println!(
        "Connecting to local node at {} (network magic {network_id})",
        socket
    );
    let config = Config::new(socket, network_id);
    let connector = Node::connect(config).await?;

    let health = connector.health().await?;
    println!("Health: {health}");

    println!("Network: {}", connector.network());

    let params = connector.protocol_parameters().await?;
    println!(
        "Protocol parameters:\n  base_fee(1): {}\n  base_fee(0): {}\n  price_mem: {}\n  price_cpu: {}\n  plutus_v3_cost_model length: {}",
        params.base_fee(1),
        params.base_fee(0),
        params.price_mem(1),
        params.price_cpu(1),
        params.plutus_v3_cost_model().len()
    );

    if let Ok(address) = std::env::var("CARDANO_NODE_QUERY_ADDRESS") {
        let address: Address<kind::Any> = address.parse()?;
        if let Some(shelley) = address.as_shelley() {
            let payment = shelley.payment();
            let delegation = shelley.delegation();
            let utxos = connector.utxos_at(&payment, delegation.as_ref()).await?;
            println!(
                "UTxOs at {address} (payment={payment:?}, delegation={delegation:?}): {}",
                utxos.len()
            );
            for (input, output) in &utxos {
                println!("  {input}: {} lovelace", output.value().lovelace());
            }
        } else {
            println!("Cannot query UTxOs: address {address} is not a Shelley address");
        }
    }

    Ok(())
}
