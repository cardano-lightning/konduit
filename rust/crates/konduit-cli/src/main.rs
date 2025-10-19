use clap::Parser;

mod cmd;
mod connector;
mod env;
mod wallet;

#[derive(clap::Parser)]
#[command(arg_required_else_help(true), version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<cmd::Cmd>,
}

fn main() {
    dotenv::dotenv().ok();
    match Cli::parse().cmd {
        Some(cmd) => cmd.run(),
        None => println!("See help"),
    };
}
