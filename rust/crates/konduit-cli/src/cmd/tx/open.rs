
use crate::metavar;
use anyhow::anyhow;
use cardano_connect::CardanoConnect;
use cardano_tx_builder as cardano;
use cardano_tx_builder::{address::kind, cbor::ToCbor};

use crate::cmd::metavar;

/// Open a channel with an adaptor and deposit some funds into it.
#[derive(Debug, clap::Args)]
#[clap(disable_version_flag(true))]
pub(crate) struct Cmd {
    /// Quantity of Lovelace to deposit into the channel
    #[clap(long, value_name = metavar::LOVELACE)]
    deposit: u64,

    /// Address of the channel's contract
    #[clap(long, value_name = metavar::ADDRESS)]
    contract: cardano::Address<kind::Any>,

    /// Address serving as user's wallet
    #[clap(long, value_name = metavar::ADDRESS)]
    wallet: cardano::Address<kind::Any>,
}

impl Cmd {
    pub(crate) async fn run(self, connector: impl CardanoConnect) -> anyhow::Result<()> {
    // let resolved_inputs = connector
    //     .utxos_at(&wallet.payment(), wallet.delegation().as_ref())
    //     .await?;

    // let from: cardano::Input = resolved_inputs
    //     .iter()
    //     .find(|(_, output)| output.value().lovelace() > args.deposit)
    //     .map(|(input, _)| input.clone())
    //     .ok_or(anyhow!("no sufficiently large output found in wallet"))?;

    // let unsigned_transaction = cardano::Transaction::build(
    //     &connector.protocol_parameters().await?,
    //     &resolved_inputs,
    //     |transaction| {
    //         transaction
    //             .with_inputs([(from.clone(), None)])
    //             .with_outputs([cardano::Output::new(
    //                 args.contract.clone(),
    //                 cardano::Value::new(args.deposit),
    //             )])
    //             .with_change_strategy(cardano::ChangeStrategy::as_last_output(
    //                 cardano::Address::from(wallet.clone()),
    //             ))
    //             .ok()
    //     },
    // )?;

    // // TODO
    // // - Signing
    // // - Submission

    // eprintln!("{unsigned_transaction}");
    // println!("{}", hex::encode(unsigned_transaction.to_cbor()));

    // Ok(())
    Ok(())
    }
}
