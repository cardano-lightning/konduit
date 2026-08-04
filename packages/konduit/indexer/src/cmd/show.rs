use std::path::PathBuf;

use cardano_sdk::VerificationKey;
use konduit_data::Tag;
use konduit_indexer::{SqliteStore, Store};

use crate::OutputFormat;

#[derive(Debug, clap::Args)]
pub struct Args {
    #[arg(long, env = "INDEXER_DB_PATH", default_value = "konduit.sqlite3")]
    pub db_path: PathBuf,

    #[arg(long)]
    pub add_vkey: VerificationKey,

    #[arg(long)]
    pub tag: Tag,

    #[arg(
        long,
        env = "INDEXER_OUTPUT_FORMAT",
        default_value = "text",
        value_enum
    )]
    pub output_format: OutputFormat,
}

pub fn run(args: Args) -> anyhow::Result<()> {
    let conn = rusqlite::Connection::open(&args.db_path)?;
    let mut store = SqliteStore::new(conn)?;
    let channel_ids = store
        .with_queries(|queries| queries.get_channel_ids_by_keytag(&args.add_vkey, &args.tag))?;
    println!(
        "Found {} channel(s) for keytag {:?} and add_vkey {:?}\n",
        channel_ids.len(),
        args.tag,
        args.add_vkey
    );
    let threads =
        store.with_queries(|queries| queries.get_threads_by_keytag(&args.add_vkey, &args.tag))?;
    match args.output_format {
        OutputFormat::Text => {
            println!("{:#?}", threads);
            Ok(())
        }
        OutputFormat::Json => {
            let s = serde_json::to_string_pretty(&threads)?;
            println!("{}", s);
            Ok(())
        }
        OutputFormat::Yaml => {
            let s = yaml_serde::to_string(&threads)?;
            println!("{}", s);
            Ok(())
        }
    }
}
