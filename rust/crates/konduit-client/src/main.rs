use anyhow::{Context, anyhow};
use cardano_tx_builder::PlutusData;
use cardano_tx_builder::{SigningKey, VerificationKey, cbor::ToCbor};
use clap::Parser;
use konduit_data::{ChequeBody, Duration, Keytag, Lock, Locked, Squash, SquashBody, Tag, Unlocked};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;
use std::io::{self, Write};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "Konduit CLI - Factorized manual interaction tool"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// URL of the Konduit server
    #[arg(
        long,
        env = "KONDUIT_SERVER_URL",
        default_value = "http://127.0.0.1:5663"
    )]
    server_url: String,

    /// Hex encoded signing key
    #[arg(long, env = "KONDUIT_SIGNING_KEY")]
    signing_key: String,

    /// Hex encoded Tag. Required.
    #[arg(long, env = "KONDUIT_TAG")]
    tag: String,

    /// Optional LND REST URL
    #[arg(long, env = "LND_BASE_URL")]
    lnd_url: Option<String>,

    /// Optional LND Macaroon (Hex)
    #[arg(long, env = "LND_MACAROON")]
    lnd_macaroon: Option<String>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    yes: bool,
}

#[derive(clap::Subcommand)]
enum Commands {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuoteResponse {
    pub index: u64,
    pub amount: u64,
    pub relative_timeout: u64,
    pub routing_fee: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SquashResponse {
    Complete,
    Incomplete(SquashProposal),
    Stale(SquashProposal),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquashProposal {
    pub proposal: SquashBody,
    pub current: Squash,
    pub unlockeds: Vec<Unlocked>,
    pub lockeds: Vec<Locked>,
}

impl fmt::Display for QuoteResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n--- Quote ---")?;
        writeln!(f, "Index          :  {}", self.index)?;
        writeln!(f, "Amount    (Ada):  {}", self.amount)?;
        writeln!(f, "Fee      (msat):  {}", self.routing_fee)?;
        writeln!(
            f,
            "Timeout (hours):  {:.2}",
            self.relative_timeout / (1000 * 60 * 60)
        )
    }
}

struct Config {
    server_url: String,
    client: Client,
    key: SigningKey,
    tag: Tag,
    lnd: Option<LndConfig>,
    auto_confirm: bool,
}

struct LndConfig {
    url: String,
    macaroon: String,
}

impl Config {
    async fn init(cli: &Cli) -> anyhow::Result<Self> {
        let key = SigningKey::from_str(&cli.signing_key)
            .context("Invalid signing key format or length")?;
        let tag_bytes = hex::decode(&cli.tag).context("Invalid hex for tag")?;
        let tag = Tag::from(tag_bytes);

        let lnd = if let (Some(url), Some(mac)) = (&cli.lnd_url, &cli.lnd_macaroon) {
            Some(LndConfig {
                url: url.clone(),
                macaroon: mac.clone(),
            })
        } else {
            None
        };

        Ok(Self {
            server_url: cli.server_url.clone(),
            client: Client::new(),
            key,
            tag,
            lnd,
            auto_confirm: cli.yes,
        })
    }

    fn keytag(&self) -> Keytag {
        Keytag::new(VerificationKey::from(&self.key), self.tag.clone())
    }

    /// Internal helper to centralize request logic
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        include_tag: bool,
        body: Option<Value>,
        raw_body: Option<Vec<u8>>,
    ) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.server_url, path);
        let mut rb = self.client.request(method, url);

        if include_tag {
            rb = rb.header("KONDUIT", hex::encode(self.keytag().as_ref()));
        }

        if let Some(json) = body {
            rb = rb.json(&json);
        } else if let Some(bytes) = raw_body {
            rb = rb.body(bytes);
        }

        let res = rb.send().await?;
        let status = res.status();
        let text = res.text().await?;

        if !status.is_success() {
            return Err(anyhow!("Request to {} failed ({}): {}", path, status, text));
        }

        if text.is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(&text).context("Failed to parse response as JSON")
    }

    async fn show_info(&self) -> anyhow::Result<()> {
        let json = self
            .request(reqwest::Method::GET, "/info", false, None, None)
            .await?;
        println!("Server Health: {}", serde_json::to_string_pretty(&json)?);
        println!("Keytag: {}", hex::encode(self.keytag().as_ref()));
        Ok(())
    }

    async fn add_lnd_invoice(&self, msat: u64, memo: String) -> anyhow::Result<String> {
        let lnd = self
            .lnd
            .as_ref()
            .ok_or_else(|| anyhow!("LND credentials not provided"))?;
        let res = self
            .client
            .post(format!("{}/v1/invoices", lnd.url))
            .header("Grpc-Metadata-macaroon", &lnd.macaroon)
            .json(&json!({ "value_msat": msat, "memo": memo }))
            .send()
            .await?;

        let data: Value = res.json().await?;
        data["payment_request"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("LND failed to return invoice: {}", data))
    }

    async fn get_quote(&self, invoice: &str) -> anyhow::Result<QuoteResponse> {
        let val = self
            .request(
                reqwest::Method::POST,
                "/ch/quote",
                true,
                Some(json!({ "Bolt11": invoice })),
                None,
            )
            .await?;
        serde_json::from_value(val).context("Failed to coerce QuoteResponse")
    }

    async fn execute_payment(
        &self,
        invoice_str: &str,
        quote: &QuoteResponse,
    ) -> anyhow::Result<SquashResponse> {
        let invoice = invoice_str.parse::<bln_client::types::Invoice>()?;
        let lock = Lock(invoice.payment_hash);

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
        let timeout = Duration::from_millis(now + quote.relative_timeout);

        let body = ChequeBody::new(quote.index, quote.amount, timeout, lock);
        let locked = Locked::make(&self.key, &self.tag, body);

        let payload = json!({
            "invoice": invoice_str,
            "cheque_body": locked.body,
            "signature": hex::encode(locked.signature.as_ref()),
        });

        let res = self
            .request(reqwest::Method::POST, "/ch/pay", true, Some(payload), None)
            .await?;
        serde_json::from_value(res).context("Failed to parse payment response as SquashResponse")
    }

    async fn execute_squash(&self, squash_body: SquashBody) -> anyhow::Result<SquashResponse> {
        let squash = Squash::make(&self.key, &self.tag, squash_body);
        let bytes = PlutusData::from(squash).to_cbor();

        let res = self
            .request(reqwest::Method::POST, "/ch/squash", true, None, Some(bytes))
            .await?;
        serde_json::from_value(res).context("Failed to parse squash response as SquashResponse")
    }

    /// Verifies a squash proposal from the server and optionally re-executes
    async fn handle_squash_response(&self, response: SquashResponse) -> anyhow::Result<()> {
        match response {
            SquashResponse::Complete => {
                println!("Action complete.");
                Ok(())
            }
            SquashResponse::Incomplete(proposal) => {
                println!("\nReceived Incomplete Squash Response.");
                println!("Server Proposal: {:?}", proposal,);

                // Placeholder for verification steps:
                // 1. Verify signatures of all 'unlocked' items
                // 2. Verify that 'proposal' correctly sums the current state + unlockeds
                // 3. Verify 'current' matches local expectations if applicable
                println!("(Verification steps omitted...)");

                if self.auto_confirm
                    || confirm(
                        format!(
                            "Verify proposal and execute squash?\n{}",
                            serde_json::to_string_pretty(&proposal).unwrap()
                        )
                        .as_str(),
                    )?
                {
                    println!("Executing squash with proposed body...");
                    let next_res = self.execute_squash(proposal.proposal).await?;
                    // Recursively handle if it's still incomplete (e.g. multi-step sync)
                    Box::pin(self.handle_squash_response(next_res)).await
                } else {
                    Err(anyhow!("User aborted squash synchronization"))
                }
            }
            SquashResponse::Stale(proposal) => {
                eprintln!("Squash stale.");
                println!("{}", serde_json::to_string_pretty(&proposal).unwrap());
                Ok(())
            }
        }
    }

    async fn handle_full_pay_flow(&self, invoice: String) -> anyhow::Result<()> {
        let quote = self.get_quote(&invoice).await?;
        println!("{}", quote);

        if !self.auto_confirm && !confirm("Proceed with payment?")? {
            return Ok(());
        }

        let res = self.execute_payment(&invoice, &quote).await?;
        self.handle_squash_response(res).await
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    let cli = Cli::parse();
    let config = Config::init(&cli).await?;

    match cli.command {
        Commands::Info => config.show_info().await?,
        Commands::AddInvoice { amount_msat, memo } => {
            let inv = config.add_lnd_invoice(amount_msat, memo).await?;
            println!("{}", inv);
        }
        Commands::Quote { invoice } => {
            let quote = config.get_quote(&invoice).await?;
            println!("{}", quote);
        }
        Commands::Pay { invoice } => config.handle_full_pay_flow(invoice).await?,
        Commands::Squash => {
            let res = config.execute_squash(SquashBody::zero()).await?;
            config.handle_squash_response(res).await?;
        }
    }

    Ok(())
}

fn confirm(prompt: &str) -> anyhow::Result<bool> {
    print!("\n{} (Type 'Y' to confirm): ", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_uppercase() == "Y")
}
