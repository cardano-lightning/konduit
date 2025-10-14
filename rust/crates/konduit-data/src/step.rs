use anyhow::{Result, anyhow};
use pallas_primitives::PlutusData;

use super::{
    mix::Mixs,
    plutus::{self, PData, constr},
    squash::Squash,
    unlocked::Unlockeds,
};

#[derive(Debug, Clone)]
pub struct Unpends(pub Vec<Vec<u8>>);
impl PData for Unpends {
    fn to_plutus_data(&self) -> PlutusData {
        PData::to_plutus_data(&self.0)
    }

    fn from_plutus_data(data: &PlutusData) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Self(PData::from_plutus_data(data)?))
    }
}

#[derive(Debug, Clone)]
pub enum Step {
    Cont(Cont),
    Eol(Eol),
}

impl PData for Step {
    fn to_plutus_data(&self) -> PlutusData {
        match self {
            Self::Cont(x) => constr(0, vec![x.to_plutus_data()]),
            Self::Eol(x) => constr(1, vec![x.to_plutus_data()]),
        }
    }

    fn from_plutus_data(data: &PlutusData) -> Result<Self>
    where
        Self: Sized,
    {
        let (constr_index, v) = &plutus::unconstr(data)?;
        match constr_index {
            0 => match &v[..] {
                [x] => Ok(Self::Cont(PData::from_plutus_data(x)?)),
                _ => Err(anyhow!("bad length")),
            },
            1 => match &v[..] {
                [x] => Ok(Self::Eol(PData::from_plutus_data(x)?)),
                _ => Err(anyhow!("bad length")),
            },
            _ => Err(anyhow!("Bad constr tag")),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Cont {
    Add,
    Sub(Squash, Unlockeds),
    Close,
    Respond(Squash, Mixs),
    Unlock(Unpends),
    Expire(Unpends),
}

#[derive(Debug, Clone)]
pub enum Eol {
    End,
    Elapse,
}

impl PData for Eol {
    fn to_plutus_data(&self) -> PlutusData {
        match self {
            Self::End => constr(0, vec![]),
            Self::Elapse => constr(1, vec![]),
        }
    }

    fn from_plutus_data(data: &PlutusData) -> Result<Self>
    where
        Self: Sized,
    {
        let (constr_index, v) = &plutus::unconstr(data)?;
        match constr_index {
            0 => match &v[..] {
                [] => Ok(Self::End),
                _ => Err(anyhow!("bad length")),
            },
            1 => match &v[..] {
                [] => Ok(Self::Elapse),
                _ => Err(anyhow!("bad length")),
            },
            _ => Err(anyhow!("Bad constr tag")),
        }
    }
}

impl Cont {
    pub fn new_add() -> Self {
        Self::Add
    }

    pub fn new_sub(squash: Squash, unlockeds: Unlockeds) -> Self {
        Self::Sub(squash, unlockeds)
    }

    pub fn new_close() -> Self {
        Self::Close
    }

    pub fn new_respond(squash: Squash, mixs: Mixs) -> Self {
        Self::Respond(squash, mixs)
    }

    pub fn new_unlock(unpends: Unpends) -> Self {
        Self::Unlock(unpends)
    }

    pub fn new_expire(unpends: Unpends) -> Self {
        Self::Expire(unpends)
    }
}

impl PData for Cont {
    fn to_plutus_data(self: &Self) -> PlutusData {
        match &self {
            Self::Add => constr(0, vec![]),
            Self::Sub(squash, unlockeds) => {
                constr(1, vec![squash.to_plutus_data(), unlockeds.to_plutus_data()])
            }
            Self::Close => constr(2, vec![]),
            Self::Respond(squash, mixs) => {
                constr(3, vec![squash.to_plutus_data(), mixs.to_plutus_data()])
            }
            Self::Unlock(unpends) => {
                constr(4, unpends.0.iter().map(PData::to_plutus_data).collect())
            }
            Self::Expire(unpends) => {
                constr(4, unpends.0.iter().map(PData::to_plutus_data).collect())
            }
        }
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
        let (constr_index, v) = &plutus::unconstr(d)?;
        match constr_index {
            0 => match &v[..] {
                [] => Ok(Self::new_add()),
                _ => Err(anyhow!("bad length")),
            },
            1 => match &v[..] {
                [a, b] => Ok(Self::new_sub(
                    PData::from_plutus_data(&a)?,
                    PData::from_plutus_data(&b)?,
                )),
                _ => Err(anyhow!("bad length")),
            },
            2 => match &v[..] {
                [] => Ok(Self::new_close()),
                _ => Err(anyhow!("bad length")),
            },
            3 => match &v[..] {
                [a, b] => Ok(Self::new_respond(
                    PData::from_plutus_data(&a)?,
                    PData::from_plutus_data(&b)?,
                )),
                _ => Err(anyhow!("bad length")),
            },
            4 => Ok(Self::new_unlock(Unpends(
                v.iter()
                    .map(PData::from_plutus_data)
                    .collect::<Result<Vec<Vec<u8>>>>()?,
            ))),
            5 => Ok(Self::new_expire(Unpends(
                v.iter()
                    .map(PData::from_plutus_data)
                    .collect::<Result<Vec<Vec<u8>>>>()?,
            ))),
            _ => Err(anyhow!("Bad constr tag")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Steps(pub Vec<Step>);

impl PData for Steps {
    fn to_plutus_data(self: &Self) -> PlutusData {
        PData::to_plutus_data(&self.0)
    }

    fn from_plutus_data(d: &PlutusData) -> Result<Self> {
        let v = plutus::unlist(d)?;
        let x: Vec<Step> = v
            .iter()
            .map(|x| PData::from_plutus_data(x))
            .collect::<Result<Vec<_>>>()?;
        Ok(Self(x))
    }
}
