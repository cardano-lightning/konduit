//! Adapts `cardano_wallet::Cmd` to a `Session`.

use cardano_connector::CardanoConnector;
use cardano_wallet::{Cmd, Wallet};

use crate::Session;

use super::cmd::print_json;

pub async fn run<C: CardanoConnector, W: Wallet>(
    session: &mut Session<C, W>,
    cmd: &Cmd,
) -> anyhow::Result<()> {
    if matches!(cmd, Cmd::Init) {
        anyhow::bail!("`wallet init` isn't meaningful for a session - use the top-level `init`");
    }
    let value = cmd
        .run(session, session.protocol_parameters())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    print_json(&value)
}
