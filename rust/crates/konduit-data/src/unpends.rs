use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone)]
pub struct Unpends(pub Vec<Vec<u8>>);

impl<'a> TryFrom<&PlutusData<'a>> for Unpends {
    type Error = Error;

    fn try_from(data: &PlutusData<'a>) -> Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(|x| {
                x.as_bytes()
                    .and_then(|x| Some(x.to_vec()))
                    .ok_or(anyhow!("Expect bytes"))
            })
            .collect::<Result<Vec<Vec<u8>>>>()?;
        Ok(Unpends(inner))
    }
}

impl<'a> From<Unpends> for PlutusData<'a> {
    fn from(value: Unpends) -> Self {
        Self::list(value.0.into_iter().map(|x| Self::bytes(x)))
    }
}
