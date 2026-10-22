use crate::config::admin::Config;
use crate::output::{self, OutputJson};

/// Show
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Show config. This is a parsed version of env
    Config {
        /// Emit machine-readable JSON instead of human-readable text
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
    /// Show address.
    Address {
        /// Emit machine-readable JSON instead of human-readable text
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
    /// Show tip
    Tip {
        // Verbosity
        #[arg(long, default_value_t = false)]
        verbose: bool,
        /// Emit machine-readable JSON instead of human-readable text
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
}

impl Cmd {
    pub(crate) async fn run(self, config: &Config) -> anyhow::Result<()> {
        match self {
            Cmd::Config { output_json } => render_config(config, OutputJson(output_json)),
            Cmd::Address { output_json } => render_address(config, OutputJson(output_json)),
            Cmd::Tip {
                verbose,
                output_json,
            } => render_tip(config, verbose, OutputJson(output_json)).await,
        }
    }
}

fn render_config(config: &Config, output: OutputJson) -> anyhow::Result<()> {
    if output.is_json() {
        // The Config struct itself derives/uses serde for the Connector but the wallet
        // and host_address are not serde-friendly. We expose only the JSON-friendly
        // view through `output::AddressOutput` and a small config summary.
        let address = output::AddressOutput::from_admin(config)?;
        output::render(output, &address)?;
    } else {
        print!("{}", config);
    }
    Ok(())
}

fn render_address(config: &Config, output: OutputJson) -> anyhow::Result<()> {
    if output.is_json() {
        let dto = output::AddressOutput::from_admin(config)?;
        output::render(output, &dto)?;
    } else {
        let network_id = config.connector.network().into();
        print!(
            "{}",
            config.wallet.to_verification_key().to_address(network_id)
        );
    }
    Ok(())
}

async fn render_tip(config: &Config, verbose: bool, output: OutputJson) -> anyhow::Result<()> {
    let connector = config.connector.connector().await?;
    let tip = crate::tip::Admin::new(&connector, config).await?;
    if output.is_json() {
        let dto = build_admin_tip_output(&tip)?;
        output::render(output, &dto)?;
    } else if verbose {
        println!("{:#}", tip);
    } else {
        println!("{}", tip);
    }
    Ok(())
}

fn build_admin_tip_output(tip: &crate::tip::Admin) -> anyhow::Result<output::AdminTipOutput> {
    // Build a one-element reference_script option so WalletSummary & AdminTipOutput stay in sync.
    let reference_script_utxo = tip.reference_script().cloned();
    let reference_script = reference_script_utxo
        .as_ref()
        .map(|(i, o)| output::ReferenceScriptDto::from_utxo(i, o))
        .transpose()?;
    let wallet_summary = output::WalletSummary::from_utxos(tip.wallet(), &reference_script_utxo)?;
    Ok(output::AdminTipOutput {
        wallet: wallet_summary,
        reference_script,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::test_helpers;

    #[test]
    fn address_json_output_roundtrips() {
        let config = test_helpers::admin();
        let dto = output::AddressOutput::from_admin(&config).unwrap();
        let json = serde_json::to_string(&dto).unwrap();
        let back: output::AddressOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
        assert!(json.contains("\"address\""));
    }

    #[test]
    fn config_render_text_does_not_panic() {
        render_config(&test_helpers::admin(), OutputJson(false)).unwrap();
    }

    #[test]
    fn config_render_json_emits_valid_json() {
        render_config(&test_helpers::admin(), OutputJson(true)).unwrap();
    }

    #[test]
    fn admin_tip_summary_is_serializable() {
        use cardano_sdk::address::kind;
        use cardano_sdk::{Hash, Input, Output, Value};
        let sk = cardano_sdk::SigningKey::from([9; 32]);
        let any_address: cardano_sdk::Address<kind::Any> = sk
            .to_verification_key()
            .to_address(cardano_sdk::Network::PREVIEW.into())
            .into();
        let utxos = std::collections::BTreeMap::from([(
            Input::new(Hash::<32>::from([1; 32]), 0),
            Output::new(any_address, Value::new(30 * 1_000_000)),
        )]);
        let summary = output::WalletSummary::from_utxos(&utxos, &None).unwrap();
        let dto = output::AdminTipOutput {
            wallet: summary,
            reference_script: None,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: output::AdminTipOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }
}
