use clap::Parser;

mod cmd;
mod config;
mod connector;
mod env;
mod random;
mod tip;

fn main() {
    cmd::Cmd::parse().run().unwrap();
}
