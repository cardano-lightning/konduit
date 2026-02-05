use toml;

use crate::env::admin::Env;

#[derive(Debug, Clone, clap::Args)]
pub struct Cmd {
    #[command(flatten)]
    env: Env,
}

impl Cmd {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        println!("# ./{}", Env::DEFAULT_PATH);
        println!("{:#}", toml::to_string(&self.env.fill())?);
        Ok(())
    }
}
