use cardano_connector::CardanoConnector;
use cardano_connector_direct::Blockfrost;
use cardano_sdk::{Address, VerificationKey, address::kind::Shelley};
use clap::Parser;
use http_client::{codec, transport};
use konduit_core::{Duration, Keytag, Lock, SquashBody};
use konduit_l2_dto::{info, squash};
use konduit_tx::{
    consumer::{Intent, OpenIntent},
    to_verifying_key,
};
use serde_json::json;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

#[cfg(feature = "cln")]
use crate::cln;
use crate::{
    Adaptor,
    connector::{Backend, ConnectorArgs},
    core::{SigningKey, Tag},
    l1, l2,
};
use konduit_rail::RESERVED_NAME;

#[derive(Debug, Parser)]
#[command(author, version, about = "Konduit Consumer CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Cmd,

    /// Hex encoded signing key
    #[arg(long, env = "KONDUIT_SIGNING_KEY")]
    pub signing_key: SigningKey,

    /// Hex encoded Tag. Required.
    #[arg(long, env = "KONDUIT_TAG")]
    pub tag: Tag,

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

pub fn prompt_if_incomplete(st: &squash::Response, auto_confirm: bool) -> anyhow::Result<bool> {
    if !auto_confirm && matches!(st, squash::Response::Incomplete { .. }) {
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

    #[command(flatten)]
    pub connector: ConnectorArgs,

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
    /// Add funds to the channel, on top of whatever it already locks.
    Add { amount: u64 },
    /// Close
    Close,
    /// Step the channel on, settling whatever the chain already allows.
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
    /// Get a quote for a CLN payme JSON object returned by the merchant
    Quote {
        #[arg(long)]
        payme: String,
    },
    /// Full workflow: Get quote -> Pay -> Squash using a CLN payme
    Pay {
        #[arg(long)]
        payme: String,
    },
    /// Manually squash using the latest state
    Squash,
    /// Pay a payee through this konduit's cln rail: ask the payee for a
    /// payme, then quote and pay with the rail's own instruction.
    #[cfg(feature = "cln")]
    Cln {
        /// The payee's url, where the payme is asked for.
        #[arg(long)]
        payee: String,
        #[arg(long)]
        amount: u64,
    },
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

impl Cli {
    pub async fn run(&self) -> anyhow::Result<()> {
        match &self.cmd {
            Cmd::Info => {
                let vk = self.signing_key.to_verification_key();
                let keytag = Keytag::new(&to_verifying_key(vk), &self.tag);
                println!("{}", vk);
                println!("{}", keytag);
                Ok(())
            }
            Cmd::L1(args) => self.run_l1(args).await,
            Cmd::L2(args) => self.run_l2(args).await,
        }
    }

    /// Fetches the L2 server's info, for use as a fallback when Open/Close args are omitted.
    async fn fetch_l2_info(&self) -> anyhow::Result<info::Response<info::TxHelp>> {
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
            None => Address::try_from(self.fetch_l2_info().await?.tx_help.host_address.as_str()),
        }
    }

    /// Picks the backend, then hands the connector to the backend-agnostic
    /// body below. Each arm connects its own way, so they can't share a type.
    async fn run_l1(&self, args: &L1Args) -> anyhow::Result<()> {
        match args.connector.backend {
            Backend::Blockfrost => {
                let cardano = Blockfrost::new(args.connector.blockfrost_project_id()?.to_string());
                self.run_l1_with(&cardano, args).await
            }
            #[cfg(feature = "node")]
            Backend::Node => {
                let cardano = args.connector.node().await?;
                self.run_l1_with(&cardano, args).await
            }
            #[cfg(feature = "node")]
            Backend::KupoNode => {
                let cardano = args.connector.kupo_node().await?;
                self.run_l1_with(&cardano, args).await
            }
        }
    }

    async fn run_l1_with<C: CardanoConnector>(
        &self,
        cardano: &C,
        args: &L1Args,
    ) -> anyhow::Result<()> {
        let l1 = l1::Client::new(cardano, &self.signing_key);
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

                let sub_vkey = (*sub_vkey).unwrap_or_else(|| {
                    info.as_ref().unwrap().channel_parameters.adaptor_key.into()
                });
                let close_period = close_period
                    .unwrap_or_else(|| info.as_ref().unwrap().channel_parameters.close_period);
                let ref_script_host = match ref_script_host {
                    Some(address) => address.clone(),
                    None => {
                        Address::try_from(info.as_ref().unwrap().tx_help.host_address.as_str())?
                    }
                };

                let open = OpenIntent {
                    tag: self.tag.clone(),
                    sub_vkey,
                    close_period,
                    amount: *amount,
                };

                (vec![open], BTreeMap::new(), ref_script_host)
            }

            L1Cmd::Add { amount } => (
                Vec::new(),
                BTreeMap::from([(self.tag.clone(), Intent::Add(*amount))]),
                self.resolve_ref_script_host(ref_script_host).await?,
            ),

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

        println!("{tx}");

        Ok(())
    }

    async fn run_l2(&self, args: &L2Args) -> anyhow::Result<()> {
        let server_url = self
            .server_url
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--server-url is required for l2 commands"))?;

        let vk = self.signing_key.to_verification_key();
        let keytag = Keytag::new(&to_verifying_key(vk), &self.tag);
        let server_client = client_json(server_url);
        let adaptor = Adaptor::new(server_client, Some(&keytag)).await?;
        let l2 = l2::Client::new(&adaptor, &self.signing_key);

        match &args.cmd {
            L2Cmd::Info => {
                println!("{}", serde_json::to_string_pretty(adaptor.info())?);
            }

            // Paying the adaptor itself: konduit's own reserved name,
            // and no instruction to write.
            L2Cmd::Quote { payme } => {
                let payme: cln::payme::Response = serde_json::from_str(payme)?;
                let quote = l2.quote(RESERVED_NAME, payme.amount, Vec::new()).await?;
                println!("{}", serde_json::to_string_pretty(&quote)?);
            }

            L2Cmd::Pay { payme } => {
                let payme: cln::payme::Response = serde_json::from_str(payme)?;
                let quote = l2.quote(RESERVED_NAME, payme.amount, Vec::new()).await?;

                println!("quote = {:?}", quote);

                if !self.yes && !confirm("Proceed with payment?")? {
                    return Ok(());
                }

                let outcome = cln::pay(&l2, RESERVED_NAME, &payme, &quote, Vec::new()).await?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);

                // The inbound leg is resolved by squash either way: a
                // pending payment's secret arrives as an unlocked cheque
                // in the proposal.
                let res = l2.squash_status().await?;
                let and_confirm = prompt_if_incomplete(&res, self.yes)?;

                l2.sync(res, and_confirm, |x| known_lock(&x)).await?;
            }

            L2Cmd::Squash => {
                let res = l2.squash(SquashBody::default()).await?;
                let and_confirm = prompt_if_incomplete(&res, self.yes)?;
                l2.sync(res, and_confirm, |_x: konduit_core::Lock| known_lock(&_x))
                    .await?;
            }

            #[cfg(feature = "cln")]
            L2Cmd::Cln { payee, amount } => {
                // The payme comes from the payee, not from the konduit
                // this client has a channel with.
                let payme = cln::request_payme(&client_json(payee.clone()), *amount).await?;
                let instruction = cln::quote_instruction(payme.payee)?;
                let quote = l2
                    .quote(cln::RAIL, payme.amount, instruction.clone())
                    .await?;

                println!("quote = {:?}", quote);

                if !self.yes && !confirm("Proceed with payment?")? {
                    return Ok(());
                }

                let commit = cln::commit_instruction(payme.payee, payme.amount)?;
                let outcome = cln::pay(&l2, cln::RAIL, &payme, &quote, commit).await?;
                println!("{}", serde_json::to_string_pretty(&outcome)?);

                let res = l2.squash_status().await?;
                let and_confirm = prompt_if_incomplete(&res, self.yes)?;
                l2.sync(res, and_confirm, |x| known_lock(&x)).await?;
            }
        }

        Ok(())
    }
}

pub fn known_lock(_x: &Lock) -> bool {
    false
}
