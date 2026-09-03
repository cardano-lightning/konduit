use bln_sdk::types::Invoice;
use cardano_connector_direct::Blockfrost;
use cardano_sdk::{Address, VerificationKey, address::kind::Shelley};
use clap::Parser;
use http_client::{Transport, codec, transport};
use konduit_data::{AssetCatalog, Duration, Lock, SquashBody};
use konduit_tmp::{Keytag, SessionClaimRequest, TxHelp};
use konduit_tx::consumer::{Intent, OpenIntent};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{self, Write},
    path::PathBuf,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    Adaptor,
    core::{AdaptorInfo, Hash, SigningKey, SquashStatus, Tag},
    l1, l2,
};

#[derive(Debug, Parser)]
#[command(author, version, about = "Konduit Consumer CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,

    /// Optional JSON asset catalog; must match the adaptor's catalog digest.
    #[arg(long, env = "KONDUIT_ASSET_CONFIG")]
    pub asset_config: Option<PathBuf>,

    /// Hex encoded signing key
    #[arg(long, env = "KONDUIT_SIGNING_KEY")]
    pub signing_key: SigningKey,

    /// Hex encoded Tag. Required.
    #[arg(long, env = "KONDUIT_TAG")]
    pub tag: Tag,

    /// Monotonically increasing session generation persisted by the client.
    #[arg(long, env = "KONDUIT_SESSION_GENERATION")]
    pub session_generation: Option<u64>,

    /// Hex-encoded hash of the current session backup.
    #[arg(long, env = "KONDUIT_SESSION_BACKUP_HASH")]
    pub session_backup_hash: Option<Hash<32>>,

    /// Stable public identity of this client device.
    #[arg(long, env = "KONDUIT_SESSION_DEVICE_KEY")]
    pub session_device_key: Option<VerificationKey>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,

    /// URL of the Konduit server. Required for `l2` commands; used as a fallback by
    /// `l1 open`/`l1 close` to look up whichever of sub-vkey/close-period/ref-script-host
    /// weren't supplied directly.
    #[arg(long, env = "KONDUIT_SERVER_URL")]
    pub server_url: Option<String>,
}

pub fn confirm(prompt: &str) -> anyhow::Result<bool> {
    eprint!("\n{} [y/N] ", prompt);

    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().is_empty() || input.trim().to_lowercase() == "n" {
        return Ok(false);
    }

    if input.trim().to_lowercase() == "y" {
        return Ok(true);
    }

    confirm(prompt)
}

pub fn prompt_if_incomplete(st: &SquashStatus, auto_confirm: bool) -> anyhow::Result<bool> {
    if !auto_confirm && matches!(st, SquashStatus::Incomplete { .. }) {
        println!("{}", serde_json::to_string_pretty(st).unwrap());
        confirm("Verify proposal and execute squash?")
    } else {
        Ok(auto_confirm)
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Show own info. Derived locally from the signing key/tag; touches no network.
    Info,
    /// Cardano L1 actions (channel lifecycle on-chain).
    L1(L1Args),
    /// Konduit server L2 actions.
    L2(L2Args),
}

#[derive(Debug, clap::Args)]
pub struct L1Args {
    #[command(subcommand)]
    pub cmd: L1Cmd,

    /// For L1, only the blockfrost connector is supported.
    #[arg(long, env = "KONDUIT_BLOCKFROST_PROJECT_ID")]
    pub blockfrost_project_id: String,

    /// Reference script host address. If omitted, it's fetched from the L2 server.
    #[arg(long, env = "KONDUIT_HOST_ADDRESS")]
    ref_script_host: Option<Address<Shelley>>,
}

#[derive(Debug, clap::Subcommand)]
pub enum L1Cmd {
    /// Channel
    Channels,
    /// Open
    Open {
        amount: u64,

        /// Sub-key to open the channel with. If omitted, it's fetched from the L2 server.
        #[arg(long = "sub-vkey")]
        sub_vkey: Option<VerificationKey>,

        /// Close period to open the channel with. If omitted, it's fetched from the L2 server.
        #[arg(long = "close-period")]
        close_period: Option<Duration>,
    },
    /// Close
    Close,
    /// Close
    Step,
}

#[derive(Debug, clap::Args)]
pub struct L2Args {
    #[command(subcommand)]
    pub cmd: L2Cmd,
}

#[derive(Debug, clap::Subcommand)]
pub enum L2Cmd {
    /// Show info about the server
    Info,
    /// Get a quote for a lightning invoice
    Quote { invoice: String },
    /// Full workflow: Get quote -> Pay -> Squash
    Pay { invoice: String },
    /// Manually squash using the latest state
    Squash,
}

pub fn transport() -> transport::Reqwest {
    http_client::transport::Reqwest::new(Some(web_time::Duration::from_secs(10)))
}

pub fn client_cbor(base_url: String) -> http_client::Client<transport::Reqwest, codec::Cbor> {
    http_client::Client::new(transport(), codec::Cbor, base_url)
}

pub fn client_json(base_url: String) -> http_client::Client<transport::Reqwest, codec::Json> {
    http_client::Client::new(transport(), codec::Json, base_url)
}

async fn claim_cli_session<T: Transport>(
    cli: &Cli,
    adaptor: &mut Adaptor<T>,
    last_timestamp: &mut u64,
) -> anyhow::Result<()> {
    let generation = cli
        .session_generation
        .ok_or_else(|| anyhow::anyhow!("--session-generation is required"))?;
    if generation == 0 {
        anyhow::bail!("--session-generation must be at least 1");
    }
    let backup_hash = cli
        .session_backup_hash
        .ok_or_else(|| anyhow::anyhow!("--session-backup-hash is required"))?;
    let device_key = cli
        .session_device_key
        .ok_or_else(|| anyhow::anyhow!("--session-device-key is required"))?;
    let adaptor_key: [u8; 32] = adaptor.info().channel_parameters.adaptor_key.into();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let now = now.max(last_timestamp.saturating_add(1));
    let claim = SessionClaimRequest::signed(
        &cli.signing_key,
        adaptor_key,
        generation,
        backup_hash.into(),
        device_key.into(),
        now,
    );
    adaptor.claim_session(&claim).await?;
    *last_timestamp = now;
    Ok(())
}

impl Cli {
    pub async fn run(&self) -> anyhow::Result<()> {
        match &self.cmd {
            Cmd::Info => {
                let vk = self.signing_key.to_verification_key();
                let keytag = Keytag::new(&vk, &self.tag);
                println!("{}", vk);
                println!("{}", keytag);
                Ok(())
            }
            Cmd::L1(args) => self.run_l1(args).await,
            Cmd::L2(args) => self.run_l2(args).await,
        }
    }

    /// Fetches the L2 server's info, for use as a fallback when Open/Close args are omitted.
    async fn fetch_l2_info(&self) -> anyhow::Result<AdaptorInfo<TxHelp>> {
        let server_url = self.server_url.clone().ok_or_else(|| {
            anyhow::anyhow!("--server-url is required to look up values not supplied directly")
        })?;
        let adaptor = Adaptor::new(client_json(server_url), None).await?;
        Ok(adaptor.info().clone())
    }

    /// Resolves ref_script_host from the arg, or the L2 server if omitted.
    async fn resolve_ref_script_host(
        &self,
        given: &Option<Address<Shelley>>,
    ) -> anyhow::Result<Address<Shelley>> {
        match given {
            Some(addr) => Ok(addr.clone()),
            None => Ok(self.fetch_l2_info().await?.tx_help.host_address.clone()),
        }
    }

    async fn run_l1(&self, args: &L1Args) -> anyhow::Result<()> {
        let cardano = Blockfrost::new(args.blockfrost_project_id.clone());
        let l1 = l1::Client::new(&cardano, &self.signing_key);
        let ref_script_host = &args.ref_script_host;

        // Channels is a plain query, not an execute() call - handle it and return early.
        // Open/Close/Step all boil down to: build the intents, then hit the same execute().
        let (open_intents, close_intents, ref_script_host) = match &args.cmd {
            L1Cmd::Channels => {
                let channels = l1.channels(None).await?.collect::<Vec<_>>();
                let json_channels: Vec<serde_json::Value> = channels
                    .into_iter()
                    .map(|c| {
                        json!({
                            "id": c.constants(),
                            "stage": &c.stage(),
                            "amount": &c.amount(),
                        })
                    })
                    .collect();

                println!("{}", serde_json::to_string_pretty(&json_channels)?);
                return Ok(());
            }

            L1Cmd::Open {
                amount,
                sub_vkey,
                close_period,
            } => {
                let info =
                    if sub_vkey.is_none() || close_period.is_none() || ref_script_host.is_none() {
                        Some(self.fetch_l2_info().await?)
                    } else {
                        None
                    };

                let sub_vkey = (*sub_vkey)
                    .unwrap_or_else(|| info.as_ref().unwrap().channel_parameters.adaptor_key);
                let close_period = close_period
                    .unwrap_or_else(|| info.as_ref().unwrap().channel_parameters.close_period);
                let ref_script_host = ref_script_host
                    .clone()
                    .unwrap_or_else(|| info.as_ref().unwrap().tx_help.host_address.clone());

                let open = OpenIntent {
                    tag: self.tag.clone(),
                    sub_vkey,
                    close_period,
                    amount: *amount,
                };

                (vec![open], BTreeMap::new(), ref_script_host)
            }

            L1Cmd::Close => (
                Vec::new(),
                BTreeMap::from([(self.tag.clone(), Intent::Close)]),
                self.resolve_ref_script_host(ref_script_host).await?,
            ),

            L1Cmd::Step => (
                Vec::new(),
                BTreeMap::new(),
                self.resolve_ref_script_host(ref_script_host).await?,
            ),
        };

        let tx = l1
            .execute(
                &self.signing_key,
                None,
                open_intents,
                close_intents,
                &ref_script_host,
            )
            .await?;

        println!("{}", serde_json::to_string_pretty(&tx)?);

        Ok(())
    }

    async fn run_l2(&self, args: &L2Args) -> anyhow::Result<()> {
        let server_url = self
            .server_url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--server-url is required for l2 commands"))?;

        let vk = self.signing_key.to_verification_key();
        let keytag = Keytag::new(&vk, &self.tag);
        let server_client = client_json(server_url);
        let mut adaptor = Adaptor::new(server_client, Some(&keytag)).await?;
        if !matches!(args.cmd, L2Cmd::Info) {
            let catalog_digest = AssetCatalog::load(self.asset_config.as_deref())?.digest()?;
            if adaptor.info().asset_catalog_digest.as_deref() != Some(&catalog_digest) {
                anyhow::bail!("asset catalog does not match adaptor discovery");
            }
        }
        let parsed_invoice = match &args.cmd {
            L2Cmd::Quote { invoice } | L2Cmd::Pay { invoice } => {
                Some(invoice.parse::<Invoice>()?)
            }
            _ => None,
        };
        let mut last_claim_timestamp = 0;
        let protected = matches!(
            args.cmd,
            L2Cmd::Quote { .. } | L2Cmd::Pay { .. } | L2Cmd::Squash
        );
        if protected {
            claim_cli_session(self, &mut adaptor, &mut last_claim_timestamp).await?;
        }

        match &args.cmd {
            L2Cmd::Info => {
                println!("{}", serde_json::to_string_pretty(adaptor.info())?);
            }

            L2Cmd::Quote { .. } => {
                let invoice = parsed_invoice.unwrap();
                let quote = l2::Client::new(&adaptor, &self.signing_key)
                    .quote(&invoice)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&quote)?);
            }

            L2Cmd::Pay { .. } => {
                let invoice = parsed_invoice.unwrap();
                let quote = l2::Client::new(&adaptor, &self.signing_key)
                    .quote(&invoice)
                    .await?;

                println!("quote = {:?}", quote);

                if !self.yes && !confirm("Proceed with payment?")? {
                    return Ok(());
                }

                claim_cli_session(self, &mut adaptor, &mut last_claim_timestamp).await?;
                let res = l2::Client::new(&adaptor, &self.signing_key)
                    .pay(&invoice, &quote)
                    .await?;
                let and_confirm = prompt_if_incomplete(&res, self.yes)?;
                if and_confirm {
                    claim_cli_session(self, &mut adaptor, &mut last_claim_timestamp).await?;
                }
                l2::Client::new(&adaptor, &self.signing_key)
                    .sync(res, and_confirm, |x| known_lock(&x))
                    .await?;
            }

            L2Cmd::Squash => {
                let res = l2::Client::new(&adaptor, &self.signing_key)
                    .squash(SquashBody::default())
                    .await?;
                let and_confirm = prompt_if_incomplete(&res, self.yes)?;
                if and_confirm {
                    claim_cli_session(self, &mut adaptor, &mut last_claim_timestamp).await?;
                }
                l2::Client::new(&adaptor, &self.signing_key)
                    .sync(res, and_confirm, |_x: konduit_data::Lock| known_lock(&_x))
                    .await?;
            }
        }

        Ok(())
    }
}

pub fn known_lock(_x: &Lock) -> bool {
    false
}
