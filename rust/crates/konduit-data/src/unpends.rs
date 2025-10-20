use anyhow::anyhow;
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone)]
pub struct Unpends(pub Vec<Vec<u8>>);

impl<'a> TryFrom<&PlutusData<'a>> for Unpends {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let list = data.as_list().ok_or(anyhow!("Expect list"))?;
        let inner = list
            .into_iter()
            .map(|x| {
                x.as_bytes()
                    .map(|x| x.to_vec())
                    .ok_or(anyhow!("Expect bytes"))
            })
            .collect::<anyhow::Result<Vec<Vec<u8>>>>()?;
        Ok(Unpends(inner))
    }
}

impl<'a> From<Unpends> for PlutusData<'a> {
    fn from(value: Unpends) -> Self {
        Self::list(value.0.into_iter().map(Self::bytes))
    }
}

impl Unpends {
    // TODO :: I'm pretty sure we support this
    pub fn truncate(mut self) -> Self {
        while let Some(last) = self.0.last() {
            if last.is_empty() {
                self.0.pop();
            } else {
                break;
            }
        }
        self
    }
}
