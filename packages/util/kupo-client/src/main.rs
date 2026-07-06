#[cfg(feature = "cli")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use kupo_client::{Client, MatchFilters, Pattern};

    dotenvy::dotenv().ok();
    let args = kupo_client::cli::Args::parse();

    let base = format!("http://{}:{}", args.host, args.port);
    eprintln!("Querying Kupo at {base}…");

    let client = Client::new(&base)?;
    let filters = MatchFilters::new().with_resolve_hashes(args.resolve_hashes);

    let mut matches = if let Some(pattern) = args.pattern.as_deref() {
        // `Pattern::Address` is the catch-all variant: it round-trips any
        // string Kupo accepts (wildcard, address, asset id, output ref).
        client
            .matches(&Pattern::Address(pattern.to_string()), &filters)
            .await?
    } else {
        client.all_matches(&filters).await?
    };

    matches.truncate(args.limit);

    println!("{}", serde_json::to_string_pretty(&matches)?);
    Ok(())
}

#[cfg(not(feature = "cli"))]
fn main() {
    panic!("This binary requires the 'cli' feature to be enabled.");
}
