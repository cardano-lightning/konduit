use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

#[derive(Debug, Clone)]
pub struct Index(pub u64);

impl<'a> TryFrom<PlutusData<'a>> for Index {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_integer() {
            Some(int) => Ok(Self(int)),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Index> for PlutusData<'a> {
    fn from(value: Index) -> Self {
        Self::integer(value.0)
    }
}

impl Index {
    pub fn incr(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Debug, Clone)]
pub struct Timestamp(pub u64);

impl<'a> TryFrom<PlutusData<'a>> for Timestamp {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_integer() {
            Some(int) => Ok(Self(int)),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Timestamp> for PlutusData<'a> {
    fn from(value: Timestamp) -> Self {
        Self::integer(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct TimeDelta(pub u64);

impl<'a> TryFrom<PlutusData<'a>> for TimeDelta {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_integer() {
            Some(int) => Ok(Self(int)),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<TimeDelta> for PlutusData<'a> {
    fn from(value: TimeDelta) -> Self {
        Self::integer(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct Amount(pub u64);

impl<'a> TryFrom<PlutusData<'a>> for Amount {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_integer() {
            Some(int) => Ok(Self(int)),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Amount> for PlutusData<'a> {
    fn from(value: Amount) -> Self {
        Self::integer(value.0)
    }
}

impl Amount {
    pub fn add(&self, x: u64) -> Self {
        Self(self.0 + x)
    }
    pub fn sub(&self, x: u64) -> Result<Self> {
        if x <= self.0 {
            Ok(Self(self.0 - x))
        } else {
            panic!("Make proper error")
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lock(pub [u8; 32]);

impl<'a> TryFrom<PlutusData<'a>> for Lock {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_bytes() {
            Some(bytes) => Ok(Self(
                <[u8; 32]>::try_from(bytes).map_err(|_| anyhow!("Bad length"))?,
            )),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Lock> for PlutusData<'a> {
    fn from(value: Lock) -> Self {
        Self::bytes(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct Secret(pub [u8; 32]);

impl<'a> TryFrom<PlutusData<'a>> for Secret {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_bytes() {
            Some(bytes) => Ok(Self(
                <[u8; 32]>::try_from(bytes).map_err(|_| anyhow!("Bad length"))?,
            )),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Secret> for PlutusData<'a> {
    fn from(value: Secret) -> Self {
        Self::bytes(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct ScriptHash(pub [u8; 28]);

impl<'a> TryFrom<PlutusData<'a>> for ScriptHash {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_bytes() {
            Some(bytes) => Ok(Self(
                <[u8; 28]>::try_from(bytes).map_err(|_| anyhow!("Bad length"))?,
            )),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<ScriptHash> for PlutusData<'a> {
    fn from(value: ScriptHash) -> Self {
        Self::bytes(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct Vkey(pub [u8; 32]);

impl<'a> TryFrom<PlutusData<'a>> for Vkey {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_bytes() {
            Some(bytes) => Ok(Self(
                <[u8; 32]>::try_from(bytes).map_err(|_| anyhow!("Bad length"))?,
            )),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Vkey> for PlutusData<'a> {
    fn from(value: Vkey) -> Self {
        Self::bytes(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct Signature(pub [u8; 64]);

impl<'a> TryFrom<PlutusData<'a>> for Signature {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_bytes() {
            Some(bytes) => Ok(Self(
                <[u8; 64]>::try_from(bytes).map_err(|_| anyhow!("Bad length"))?,
            )),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Signature> for PlutusData<'a> {
    fn from(value: Signature) -> Self {
        Self::bytes(value.0)
    }
}

#[derive(Debug, Clone)]
pub struct Tag(pub Vec<u8>);

impl<'a> TryFrom<PlutusData<'a>> for Tag {
    type Error = Error;

    fn try_from(pd: PlutusData<'a>) -> Result<Self> {
        match pd.as_bytes() {
            Some(bytes) => Ok(Self(bytes.into())),
            None => Err(anyhow!("Bad plutus data")),
        }
    }
}

impl<'a> From<Tag> for PlutusData<'a> {
    fn from(value: Tag) -> Self {
        Self::bytes(value.0)
    }
}
