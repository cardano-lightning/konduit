use crate::env;

mod adaptor;
mod admin;
mod consumer;
mod parsers;

/// A utility for constructing and driving Konduit's stages
#[derive(clap::Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), about, long_about = None)]
pub(crate) enum Cmd {
    Adaptor(WithEnv<env::adaptor::Env, adaptor::Cmd>),
    Admin(WithEnv<env::admin::Env, admin::Cmd>),
    Consumer(WithEnv<env::consumer::Env, consumer::Cmd>),
}

#[derive(clap::Parser)]
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
}
