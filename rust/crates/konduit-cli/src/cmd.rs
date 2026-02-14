use crate::env;
use clap::Parser;

mod adaptor;
mod admin;
mod consumer;
mod parsers;

/// A utility for constructing and driving Konduit's stages
#[derive(Debug, clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
pub(crate) enum Cmd {
    Adaptor(WithEnv<env::adaptor::Env, adaptor::Cmd>),
    Admin(WithEnv<env::admin::Env, admin::Cmd>),
    Consumer(WithEnv<env::consumer::Env, consumer::Cmd>),
}

#[derive(Debug, clap::Parser)]
pub struct WithEnv<E: clap::Args, C: clap::Subcommand> {
    #[command(flatten)]
    env: E,

    #[command(subcommand)]
    cmd: C,
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        match self {
            Self::Adaptor(WithEnv { env, cmd }) => cmd.run(env),
            Self::Admin(WithEnv { env, cmd }) => cmd.run(env),
            Self::Consumer(WithEnv { env, cmd }) => cmd.run(env),
        }
    }

    pub(crate) fn init() -> anyhow::Result<Self> {
        // Conditionally load any user-specific environment, based on command's names.
        let arg = std::env::args_os().nth(1);
        if let Some(arg_str) = arg.as_ref().and_then(|arg| arg.to_str()) {
            let role = format!(".env.{arg_str}");
            env::base::load_if_exists(&role)?;
        }

        // Load the global environment, after, so that the user-specific env takes precedence.
        env::base::load_if_exists(".env")?;

        Ok(Self::parse())
    }
}
