#[cfg(feature = "cli")]
async fn quote_invoice(
    client: &dyn bln_client::Api,
    invoice: &bln_client::types::Invoice,
) -> bln_client::Result<bln_client::types::QuoteResponse> {
    let route_hints = invoice
        .private_route
        .iter()
        .cloned()
        .map(|route| route.into_inner().into())
        .collect();
    client
        .quote(bln_client::types::QuoteRequest {
            amount_msat: invoice.amount_msat,
            payee: invoice.payee_compressed.serialize(),
            route_hints,
        })
        .await
}

#[cfg(feature = "cli")]
fn validate_quoted_fee(quoted_fee_msat: u64, current_fee_msat: u64) -> Result<(), String> {
    if quoted_fee_msat == current_fee_msat {
        Ok(())
    } else {
        Err(format!(
            "Quote changed (previous fee {quoted_fee_msat} msat, current fee {current_fee_msat} msat); quote again before committing"
        ))
    }
}

#[cfg(feature = "cli")]
fn validate_quoted_timeout(quoted_seconds: u64, current_seconds: u64) -> Result<(), String> {
    if quoted_seconds == current_seconds {
        Ok(())
    } else {
        Err(format!(
            "Quote timeout changed (previous {quoted_seconds}s, current {current_seconds}s); quote again before committing"
        ))
    }
}

#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use bln_client::{
        CreateInvoiceRequest,
        cli::{BlnArgs, Cmd, Config},
        types::{Invoice, PayRequest},
    };
    use clap::Parser;

    // 1. Parse CLI arguments
    dotenvy::dotenv().ok();
    let args = BlnArgs::parse();

    // 2. Map arguments to the internal Config enum
    let config = match Config::from_args(args.client) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Configuration Error: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Handle the command, initializing a payment API only for API operations.
    match args.command {
        Cmd::Health => {
            let health = config.health().await?;
            println!("{}", serde_json::to_string_pretty(&health)?);
        }
        Cmd::Invoice {
            amount_msat,
            description,
        } => {
            let client = config.build()?;
            let invoice = client
                .create_invoice(CreateInvoiceRequest {
                    amount_msat,
                    description,
                })
                .await?;
            println!("{}", serde_json::to_string_pretty(&invoice)?);
        }
        Cmd::Quote { invoice } => {
            let client = config.build()?;
            let parsed_invoice =
                Invoice::try_from(&invoice).map_err(|_| "Failed to parse BOLT11 invoice")?;
            let quote = quote_invoice(client.as_ref(), &parsed_invoice).await?;
            let max_total_msat = parsed_invoice
                .amount_msat
                .checked_add(quote.fee_msat)
                .ok_or("quoted total amount overflow")?;
            println!(
                "{}",
                serde_json::json!({
                    "backend": quote.backend,
                    "amount_msat": parsed_invoice.amount_msat,
                    "payee": hex::encode(parsed_invoice.payee_compressed.serialize()),
                    "quoted_fee_msat": quote.fee_msat,
                    "maximum_total_msat": max_total_msat,
                    "relative_timeout_seconds": quote.relative_timeout.as_secs(),
                })
            );
        }
        Cmd::Commit {
            invoice,
            quoted_fee_msat,
            quoted_timeout_seconds,
        } => {
            let client = config.build()?;
            let parsed_invoice =
                Invoice::try_from(&invoice).map_err(|_| "Failed to parse BOLT11 invoice")?;
            let quote = quote_invoice(client.as_ref(), &parsed_invoice).await?;
            validate_quoted_fee(quoted_fee_msat, quote.fee_msat)?;
            validate_quoted_timeout(quoted_timeout_seconds, quote.relative_timeout.as_secs())?;

            let maximum_total_msat = parsed_invoice
                .amount_msat
                .checked_add(quoted_fee_msat)
                .ok_or("quoted total amount overflow")?;
            println!("Invoice amount: {} msat", parsed_invoice.amount_msat);
            println!("Quoted maximum fee: {quoted_fee_msat} msat");
            println!("Maximum total debit: {maximum_total_msat} msat");
            println!(
                "Maximum route timeout: {} seconds",
                quote.relative_timeout.as_secs()
            );
            print!("Type 'commit' to authorize this payment: ");
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut confirmation = String::new();
            std::io::stdin().read_line(&mut confirmation)?;
            if confirmation.trim() != "commit" {
                println!("Payment not committed.");
                return Ok(());
            }

            let response = client
                .commit(PayRequest {
                    quote,
                    invoice: parsed_invoice,
                })
                .await?;
            if response.secret.is_none() {
                return Err("Payment returned no preimage".into());
            }
            println!("Payment committed successfully.");
            println!("Maximum debit was capped at {maximum_total_msat} msat.");
        }
    }

    Ok(())
}

#[cfg(all(test, feature = "cli"))]
mod tests {
    use super::{validate_quoted_fee, validate_quoted_timeout};

    #[test]
    fn commit_rejects_a_changed_quote() {
        assert!(validate_quoted_fee(22, 22).is_ok());
        assert!(validate_quoted_fee(22, 23).is_err());
        assert!(validate_quoted_timeout(5400, 5400).is_ok());
        assert!(validate_quoted_timeout(5400, 29400).is_err());
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    panic!("This binary requires the 'cli' feature to be enabled.");
}
