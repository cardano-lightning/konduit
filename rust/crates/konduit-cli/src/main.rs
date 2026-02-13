use clap::Parser;
use shared::DefaultPath;

mod cardano;
mod cmd;
mod config;
mod connector;
mod env;
mod shared;
mod tip;

const ROLES: [&str; 3] = [
    env::admin::Env::DEFAULT_PATH,
    env::adaptor::Env::DEFAULT_PATH,
    env::consumer::Env::DEFAULT_PATH,
];

fn main() -> anyhow::Result<()> {
    // Load environment prior to running Clap's parser, so that env var can feed into the
    // parser and usage if necessary.
    env::base::load_dotenv(".env")?;
    for role in ROLES {
        env::base::load_dotenv(role)?;
    }

    cmd::Cmd::parse().run()
}
