#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    use tokio::time::interval;

    dotenvy::dotenv().ok();
    let args = fx_client::cli::Args::parse();
    let every = args.every;

    let config = fx_client::cli::Config::from_args(args)
        .expect("Insufficient arguments to determine a valid FX client.");
    let client = config.build()?;
    let mut ticker = interval(every);

    loop {
        ticker.tick().await;
        match client.get().await {
            Ok(output) => println!("Success! Output: {:?}", output),
            Err(e) => eprintln!("Service failed: {}", e),
        }
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    panic!("This binary requires the 'cli' feature to be enabled.");
}
