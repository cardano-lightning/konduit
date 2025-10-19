mod open;
// mod send;

#[derive(clap::Subcommand)]
/// Txs. 
/// Cardano connector and wallet must be available. 
/// These are found via the environment variables or dotenv.
pub enum Cmd {
    Oops,
    /// Send
    // Send(send::Cmd),
    /// Open
    Open(open::Cmd),
}

impl Cmd {
    pub(crate) async fn run(self) -> anyhow::Result<()> {
        let connector = crate::connector::new()?;
        let env = get_env();
        let conn = cardano_connect::from_env(&env);
        let rt = Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
           println!("{:?}", conn.health().await);
        })
        match self {
            Oops => {println!("Here")
            Ok(())
            },
            Self::Open(cmd) => cmd.run(connector)
        }
    }
}
