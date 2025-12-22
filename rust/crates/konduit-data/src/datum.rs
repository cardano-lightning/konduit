use crate::{Constants, Stage};
use cardano_tx_builder::{Hash, PlutusData};

#[derive(Debug, Clone)]
pub struct Datum {
    pub own_hash: Hash<28>,
    pub constants: Constants,
    pub stage: Stage,
}

impl<'a> TryFrom<&PlutusData<'a>> for Datum {
    type Error = PlutusDataDecodeError;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let [a, b, c] = <[PlutusData; 3]>::try_from(data)?;
        let a = <&[u8; 28]>::try_from(&a)?;
        Ok(Self {
            own_hash: Hash::from(*a),
            constants: Constants::try_from(&b)?,
            stage: Stage::try_from(c)?,
        })
    }
}
impl<'a> From<PlutusData<'a>> for Datum {
    fn from(value: PlutusData<'a>) -> Self {
        Self::try_from(&value).expect("invalid datum structure")
    }
}

impl<'a> From<Datum> for PlutusData<'a> {
    fn from(value: Datum) -> Self {
        Self::list(vec![
            PlutusData::from(&<[u8; 28]>::from(value.own_hash)),
            PlutusData::from(value.constants),
            PlutusData::from(value.stage),
        ])
    }
}
