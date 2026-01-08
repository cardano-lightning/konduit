use toml;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    env: crate::env::consumer::Env,
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        println!("{:#}", toml::to_string(&self.env.fill())?);
        Ok(())
    }
}
