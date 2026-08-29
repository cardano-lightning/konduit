use clap::{Parser, Subcommand};

use crate::{keyring::Keyring, known_keys::KnownKeys, tx::Ctx};

#[derive(Debug, Parser)]
#[command(
    name = "konduit-prompt",
    about = "Interactively drive a Konduit session from the command line",
    // FIXME :: version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"),
)]
pub struct Cli {
    /// Path to a session config file.
    #[arg(long, default_value = "konduit-prompt-config.toml")]
    pub config: std::path::PathBuf,

    /// Path to a receipts file (known squash/cheque history), used to
    /// help label channels. Optional — omitted means no receipts.
    #[arg(long, default_value = "/tmp/konduit-receipts.json")]
    pub receipts: Option<std::path::PathBuf>,

    /// Force a refresh of the underlying cardano session.
    #[arg(long)]
    pub refresh: bool,

    #[command(subcommand)]
    pub cmd: Cmd,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// Write a default config to `--config`.
    Init {
        #[arg(long)]
        force: bool,
    },
    Channels {
        /// Keep the raw channel input instead of resolving it to a
        /// known-key label.
        #[arg(long)]
        no_labels: bool,
    },
    /// Interactively propose Wants against open channels, then build,
    /// sign, and submit the resulting transaction.
    Tx,
    /// Pass through to konduit-session cli
    #[command(subcommand)]
    Session(konduit_session::cli::Cmd),
}

fn default_interval() -> konduit_tx2::Interval {
    konduit_tx2::Interval::n_mins(5)
}

impl Cli {
    pub async fn run(self) -> anyhow::Result<()> {
        let Cli {
            config,
            receipts,
            refresh,
            cmd,
        } = self;

        if let Cmd::Init { force } = &cmd {
            return crate::cmd::init(&config, *force);
        }

        let config = crate::cmd::load_config(&config)?;

        if let Cmd::Session(inner) = cmd {
            return konduit_session::cli::Cli::run_with(config.session, refresh, inner).await;
        };

        let keyring = Keyring::from_config(config.keyring.clone());
        let mut known_keys = keyring.known_keys();
        known_keys.extend(config.known_keys);

        let tip_cache_path = config.session.tip_cache_path.clone();
        let addressbook_path = config.session.addressbook_path.clone();
        let mut cardano = cardano_session::Session::init(config.session).await?;

        cardano_session::cli::cmd::hydrate(
            &mut cardano,
            &tip_cache_path,
            &addressbook_path,
            refresh,
        )
        .await?;
        let mut session = konduit_session::Session::new(cardano)?;

        let result = match &cmd {
            Cmd::Init { .. } | Cmd::Session(_) => unreachable!("handled above"),
            Cmd::Channels { no_labels } => {
                // "known" = at least one of the channel's vkeys resolves to a label
                let channels: Vec<(String, konduit_tx2::channel::Channel)> = session
                    .channels()
                    .into_iter()
                    .filter_map(|(input, channel)| {
                        let label = known_keys.channel_label(channel.constants())?;
                        let key = if *no_labels {
                            format!("{input:?}")
                        } else {
                            label
                        };
                        Some((key, channel))
                    })
                    .collect();
                print_json(&channels)
            }
            Cmd::Tx => {
                let mut staged = session.stage_tx(default_interval());

                let receipts = match &receipts {
                    Some(path) => crate::receipt::load_receipts(path)?,
                    None => crate::receipt::receipts_example(),
                };
                let ctx = Ctx {
                    known_keys: known_keys.candidates(),
                    receipts,
                    ..Default::default()
                };
                crate::tx::build_staged_tx_interactively(&mut staged, &known_keys, &ctx)?;

                // Captured before `build()`, which consumes `staged`.
                let signers = staged.signers();
                let tx = session.build(staged)?;

                print_preview(&known_keys, &signers);
                if !inquire::Confirm::new("sign and submit?")
                    .with_default(false)
                    .prompt()?
                {
                    anyhow::bail!("aborted before signing");
                }

                let tx = keyring.sign_tx(tx, &known_keys, &signers)?;
                let id = session.sign_and_submit(tx).await?;
                print_json(&serde_json::json!({ "id": id.to_string() }))
            }
        };

        cardano_session::cli::cmd::persist(
            session.tip(),
            session.addressbook(),
            &tip_cache_path,
            &addressbook_path,
        );
        result
    }
}

fn print_preview(known_keys: &KnownKeys, signers: &[cardano_sdk::VerificationKey]) {
    println!("\n-- ready to sign + submit --");
    println!("  signers required: {}", signers.len());
    for vkey in signers {
        let who = known_keys
            .label_for_verification_key(vkey)
            .unwrap_or("<unrecognized key>");
        println!("    {who}");
    }
    println!();
}

fn print_json(value: &impl serde::Serialize) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
