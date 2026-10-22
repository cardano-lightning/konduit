use crate::config::consumer::Config;
use crate::output::{self, OutputJson};
use konduit_data::Tag;

/// Show
#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Show config. This is a parsed version of env
    Config {
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
    /// Show Keytag
    Keytag {
        tag: Tag,
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
    /// Show address.
    Address {
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
    /// Show tip
    Tip {
        // Verbosity
        #[arg(long, default_value_t = false)]
        verbose: bool,
        #[arg(long, default_value_t = false)]
        output_json: bool,
    },
}

impl Cmd {
    pub(crate) async fn run(self, config: &Config) -> anyhow::Result<()> {
        match self {
            Cmd::Config { output_json } => render_config(config, OutputJson(output_json)),
            Cmd::Address { output_json } => render_address(config, OutputJson(output_json)),
            Cmd::Keytag { tag, output_json } => {
                render_keytag(config, &tag, OutputJson(output_json))
            }
            Cmd::Tip {
                verbose,
                output_json,
            } => render_tip(config, verbose, OutputJson(output_json)).await,
        }
    }
}

fn render_config(config: &Config, output: OutputJson) -> anyhow::Result<()> {
    if output.is_json() {
        let dto = output::AddressOutput::from_consumer(config)?;
        output::render(output, &dto)?;
    } else {
        print!("{}", config);
    }
    Ok(())
}

fn render_address(config: &Config, output: OutputJson) -> anyhow::Result<()> {
    if output.is_json() {
        let dto = output::AddressOutput::from_consumer(config)?;
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

fn render_keytag(config: &Config, tag: &Tag, output: OutputJson) -> anyhow::Result<()> {
    let key = config.wallet.to_verification_key();
    if output.is_json() {
        let dto = output::KeytagOutput::new(&key, tag);
        output::render(output, &dto)?;
    } else {
        println!("{}", konduit_tmp::Keytag::new(&key, tag));
    }
    Ok(())
}

async fn render_tip(config: &Config, verbose: bool, output: OutputJson) -> anyhow::Result<()> {
    let connector = config.connector.connector().await?;
    let tip = crate::tip::Consumer::new(&connector, config).await?;
    if output.is_json() {
        let reference_script = tip.reference_script().cloned();
        let dto =
            output::ConsumerTipOutput::from_parts(tip.wallet(), &reference_script, tip.channels())?;
        output::render(output, &dto)?;
    } else if verbose {
        println!("{:#}", tip);
    } else {
        println!("{}", tip);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::test_helpers;

    #[test]
    fn keytag_json_roundtrips() {
        let config = test_helpers::consumer();
        let dto =
            output::KeytagOutput::new(&config.wallet.to_verification_key(), &Tag::from(vec![1; 8]));
        let json = serde_json::to_string(&dto).unwrap();
        let back: output::KeytagOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
        assert!(json.contains("\"keytag\""));
        assert!(json.contains("\"verification_key\""));
        assert!(json.contains("\"tag\""));
    }
}
