#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    use serde_json::json;
    use tokio::time::interval;

    dotenvy::dotenv().ok();
    let args = fx_client::cli::Args::parse();
    let every = args.every;

    let config = fx_client::cli::Config::from_args(args)
        .expect("Insufficient arguments to determine a valid FX client.");
    let client = config.build()?;

    let fetch_and_print = || async {
        match client.get().await {
            Ok(output) => {
                println!("{}", serde_json::to_string_pretty(&output)?);
            }
            Err(e) => {
                let log = json!({ "status": "error", "message": e.to_string() });
                eprintln!("{}", serde_json::to_string_pretty(&log)?);
            }
        }
        Ok::<(), serde_json::Error>(())
    };

    if every.is_zero() {
        fetch_and_print().await?;
        return Ok(());
    } else {
        let mut ticker = interval(every);
        loop {
            ticker.tick().await;
            fetch_and_print().await?;
        }
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    panic!("This binary requires the 'cli' feature to be enabled.");
}
