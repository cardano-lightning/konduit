use anyhow::anyhow;
use clap::Parser;
use http_client::{HttpClient as _, reqwest::HttpClient};
use konduit_client::{
    Client,
    cli::{Cli, Commands, confirm, prompt_if_incomplete},
};
use konduit_data::SquashBody;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;

    if std::fs::exists(".env.consumer")? {
        dotenvy::from_filename(".env.consumer").map_err(|err| {
            anyhow::anyhow!("{err}").context("failed to load adaptor-specific environment")
        })?;
    }

    let cli = Cli::parse();

    let client = Client::new(&cli.server_url, cli.signing_key, cli.tag).await?;

    match cli.command {
        Commands::Info => {
            println!("{}", serde_json::to_string_pretty(client.info())?);
        }

        Commands::AddInvoice { amount_msat, memo } => {
            let (lnd_url, lnd_macaroon) = cli
                .lnd_url
                .as_deref()
                .and_then(|url| Some((url, cli.lnd_macaroon.as_deref()?)))
                .ok_or_else(|| anyhow!("LND credentials not provided"))?;

            let http_client = HttpClient::new(lnd_url);

            let json: serde_json::Value = http_client
                .post_with_headers(
                    "/v1/invoices",
                    &[("Grpc-Metadata-macaroon", lnd_macaroon)],
                    serde_json::to_vec(&json!({ "value_msat": amount_msat, "memo": memo }))?,
                )
                .await?;

            json["payment_request"]
                .as_str()
                .map(|s| println!("{s}"))
                .ok_or_else(|| anyhow!("LND failed to return invoice: {}", json))?;
        }

        Commands::Quote { invoice } => {
            let quote = client.get_quote(&invoice).await?;
            println!("{}", serde_json::to_string_pretty(&quote)?);
        }

        Commands::Pay { invoice } => {
            let quote = client.get_quote(&invoice).await?;

            log::info!("quote = {}", serde_json::to_string(&quote)?);

            if !cli.yes && !confirm("Proceed with payment?")? {
                return Ok(());
            }

            let res = client.execute_payment(&invoice, &quote).await?;

            let and_confirm = prompt_if_incomplete(&res, cli.yes)?;

            client.handle_squash_response(res, and_confirm).await?;
        }

        Commands::Squash => {
            let res = client.execute_squash(SquashBody::default()).await?;
            let and_confirm = prompt_if_incomplete(&res, cli.yes)?;
            client.handle_squash_response(res, and_confirm).await?;
        }
    }

    Ok(())
}
