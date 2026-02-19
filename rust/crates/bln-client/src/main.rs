#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use bln_client::{
        cli::{BlnArgs, Cmd, Config},
        types::{Invoice, PayRequest, QuoteRequest},
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

    // 3. Build the specific backend client (e.g., LndClient)
    let client = config.build().await?;

    // 4. Handle the specific command
    match args.command {
        Cmd::Quote { amount_msat, payee } => {
            println!("--- Requesting Quote ---");

            let req = QuoteRequest {
                amount_msat,
                payee: payee.0,
            };

            match client.quote(req).await {
                Ok(res) => {
                    println!("Success!");
                    println!("  Fee: {} msat", res.fee_msat);
                    println!("  Relative Timeout: {:?}", res.relative_timeout);
                }
                Err(e) => eprintln!("API Error: {:?}", e),
            }
        }
        Cmd::Pay {
            fee_limit,
            timeout,
            invoice,
        } => {
            println!("--- Executing Payment ---");

            // Assuming Invoice can be parsed from a string (BOLT11)
            // This is a placeholder for the actual invoice parsing logic
            let parsed_invoice =
                Invoice::try_from(&invoice).map_err(|_| "Failed to parse BOLT11 invoice")?;

            let req = PayRequest {
                fee_limit,
                relative_timeout: timeout,
                invoice: parsed_invoice,
            };

            match client.pay(req).await {
                Ok(res) => {
                    println!("Payment Successful!");
                    println!("  Preimage (Secret): {:?}", res.secret.map(hex::encode));
                }
                Err(e) => eprintln!("API Error: {:?}", e),
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    panic!("This binary requires the 'cli' feature to be enabled.");
}
