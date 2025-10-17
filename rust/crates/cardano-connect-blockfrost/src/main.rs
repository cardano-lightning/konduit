use cardano_connect::CardanoConnect;
use cardano_tx_builder::address_test;
use tokio::runtime::Runtime;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let project_id = args[1].clone();
    let conn = cardano_connect_blockfrost::Blockfrost::new(project_id);
    let rt = Runtime::new().expect("Failed to create Tokio runtime");
    let address = address_test!("addr_test1wq833v4j6sfldkrr275xm9vqzkeyp65d45kzrf2qfwe3y0c9dnk2h");
    rt.block_on(async {
        println!(
            "{:?}",
            conn.utxos_at(&address.as_shelley().unwrap().payment(), &None,)
                .await
        );
    })
}
