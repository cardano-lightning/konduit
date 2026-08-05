use bln_sdk::types::Invoice;
use clap::Parser;
use http_client::{codec, transport};
use konduit_data::{Lock, SquashBody};
use konduit_tmp::Keytag;
use std::io::{self, Write};

use crate::{
    Adaptor,
    core::{SigningKey, SquashStatus, Tag},
    l2,
};

#[derive(Debug, Parser)]
#[command(author, version, about = "Konduit Consumer CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// URL of the Konduit server
    #[arg(
        long,
        env = "KONDUIT_SERVER_URL",
        default_value = "http://127.0.0.1:5663"
    )]
    pub server_url: String,

    /// Hex encoded signing key
    #[arg(long, env = "KONDUIT_SIGNING_KEY")]
    pub signing_key: SigningKey,

    /// Hex encoded Tag. Required.
    #[arg(long, env = "KONDUIT_TAG")]
    pub tag: Tag,

    /// Optional LND REST URL
    #[arg(long, env = "LND_BASE_URL")]
    pub lnd_url: Option<String>,

    /// Optional LND Macaroon (Hex)
    #[arg(long, env = "LND_MACAROON")]
    pub lnd_macaroon: Option<String>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,
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
pub enum Commands {
    /// Show own info
    OwnInfo,
    /// Show info about the server
    Info,
    /// Create an invoice on a local LND node
    AddInvoice { amount_msat: u64, memo: String },
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

impl Cli {
    pub async fn run(&self) -> anyhow::Result<()> {
        let vk = self.signing_key.to_verification_key();
        let keytag = Keytag::new(&vk, &self.tag);
        let server_client = client_json(self.server_url.clone());
        let adaptor = Adaptor::new(server_client, Some(&keytag)).await?;

        let l2 = l2::Client::new(&adaptor, &self.signing_key);

        match &self.command {
            Commands::OwnInfo => {
                println!("{}", vk);
                println!("{}", keytag);
            }

            Commands::Info => {
                println!("{}", serde_json::to_string_pretty(adaptor.info())?);
            }

            Commands::AddInvoice { .. } => {
                todo!("Not yet impl")
                //     let (lnd_url, lnd_macaroon) = self
                //         .lnd_url
                //         .as_deref()
                //         .and_then(|url| Some((url, cli.lnd_macaroon.as_deref()?)))
                //         .ok_or_else(|| anyhow!("LND credentials not provided"))?;

                //     let http_client = client_json(lnd_url.to_string());

                //     let json: serde_json::Value = http_client
                //         .post_with_headers(
                //             "/v1/invoices",
                //             &[("Grpc-Metadata-macaroon", lnd_macaroon)],
                //             serde_json::to_vec(&json!({ "value_msat": amount_msat, "memo": memo }))?,
                //         )
                //     .await?;

                //     json["payment_request"]
                //         .as_str()
                //         .map(|s| println!("{s}"))
                //         .ok_or_else(|| anyhow!("LND failed to return invoice: {}", json))?;
            }

            Commands::Quote { invoice } => {
                let invoice = invoice.parse::<Invoice>()?;
                let quote = l2.quote(&invoice).await?;
                println!("{}", serde_json::to_string_pretty(&quote)?);
            }

            Commands::Pay { invoice } => {
                let invoice = invoice.parse::<Invoice>()?;
                let quote = l2.quote(&invoice).await?;

                println!("quote = {:?}", quote);

                if !self.yes && !confirm("Proceed with payment?")? {
                    return Ok(());
                }

                let res = l2.pay(&invoice, &quote).await?;

                let and_confirm = prompt_if_incomplete(&res, self.yes)?;

                l2.sync(res, and_confirm, |x| known_lock(&x)).await?;
            }

            Commands::Squash => {
                let res = l2.squash(SquashBody::default()).await?;
                let and_confirm = prompt_if_incomplete(&res, self.yes)?;
                l2.sync(res, and_confirm, |_x: konduit_data::Lock| known_lock(&_x))
                    .await?;
            }
        }

        Ok(())
    }
}

pub fn known_lock(_x: &Lock) -> bool {
    false
}
