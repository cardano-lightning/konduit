use clap::Parser;

mod cardano;
mod cmd;
mod config;
mod connector;
mod env;
mod tip;

fn main() {
    cmd::Cmd::parse().run().unwrap();
}
