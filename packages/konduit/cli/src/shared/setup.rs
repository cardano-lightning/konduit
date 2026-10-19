use crate::shared::{DefaultPath, Fill};
use std::io::IsTerminal;

pub trait Setup {
    fn setup(self) -> anyhow::Result<()>
    where
        Self: DefaultPath + Fill<Error = anyhow::Error> + serde::Serialize + Sized,
    {
        if std::io::stdout().is_terminal() {
            println!("./{}", Self::DEFAULT_PATH);
        }
        println!("{:#}", toml::to_string(&self.fill()?)?.replace(" = ", "="));
        Ok(())
    }
}

// fn setup_from_env<T: DefaultPath + Fill<Error = anyhow::Error> + serde::Serialize + Sized>(
//     env: T,
// ) -> anyhow::Result<()> {
//     if std::io::stdout().is_terminal() {
//         println!("./{}", T::DEFAULT_PATH);
//     }
//     println!("{:#}", toml::to_string(&env.fill()?)?.replace(" = ", "="));
//     Ok(())
// }
