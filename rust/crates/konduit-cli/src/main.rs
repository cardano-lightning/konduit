mod cardano;
mod cmd;
mod config;
mod connector;
mod env;
mod shared;
mod tip;

fn main() -> anyhow::Result<()> {
    cmd::Cmd::init()?.run()
}
