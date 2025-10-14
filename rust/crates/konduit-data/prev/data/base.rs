use crate::utils::v2a;

use super::plutus::{self, PData};

#[derive(Debug, Clone)]
pub struct Index(pub u64);

impl PData for Index {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::int(self.0.into())
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(plutus::unint(data)?.try_into()?))
    }
}

impl Index {
    pub fn incr(&self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Debug, Clone)]
pub struct Timestamp(pub u64);

impl PData for Timestamp {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::int(self.0.into())
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(plutus::unint(data)?.try_into()?))
    }
}

#[derive(Debug, Clone)]
pub struct TimeDelta(pub u64);

impl PData for TimeDelta {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::int(self.0.into())
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(plutus::unint(data)?.try_into()?))
    }
}

#[derive(Debug, Clone)]
pub struct Amount(pub u64);

impl PData for Amount {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::int(self.0.into())
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(plutus::unint(data)?.try_into()?))
    }
}

impl Amount {
    fn add(&self, x: u64) -> Self {
        Self(self.0 + x)
    }
    fn sub(&self, x: u64) -> anyhow::Result<Self> {
        if x <= self.0 {
            Ok(Self(self.0 - x))
        } else {
            panic!("Make proper error")
        }
    }
}

#[derive(Debug, Clone)]
pub struct Hash32([u8; 32]);

impl PData for Hash32 {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::bytes(&self.0)
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(v2a(plutus::unbytes(data)?)?))
    }
}

#[derive(Debug, Clone)]
pub struct Hash28(pub [u8; 28]);

impl PData for Hash28 {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::bytes(&self.0)
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(v2a(plutus::unbytes(data)?)?))
    }
}

#[derive(Debug, Clone)]
pub struct Secret(pub [u8; 32]);

impl PData for Secret {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::bytes(&self.0)
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(v2a(plutus::unbytes(data)?)?))
    }
}

#[derive(Debug, Clone)]
pub struct VKey(pub [u8; 32]);

impl PData for VKey {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::bytes(&self.0)
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(v2a(plutus::unbytes(data)?)?))
    }
}

#[derive(Debug, Clone)]
pub struct Signature(pub [u8; 64]);

impl PData for Signature {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::bytes(&self.0)
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(v2a(plutus::unbytes(data)?)?))
    }
}

#[derive(Debug, Clone)]
pub struct Tag(pub Vec<u8>);

impl PData for Tag {
    fn to_plutus_data(&self) -> uplc::PlutusData {
        plutus::bytes(&self.0)
    }

    fn from_plutus_data(data: &uplc::PlutusData) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(plutus::unbytes(data)?))
    }
}
