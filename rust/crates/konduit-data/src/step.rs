use anyhow::{Error, Result, anyhow};
use cardano_tx_builder::PlutusData;

use crate::{mixed_cheques::MixedCheques, squash::Squash, unlockeds::Unlockeds, unpends::Unpends};

#[derive(Debug, Clone)]
pub enum Step {
    Cont(Cont),
    Eol(Eol),
}

impl Step {
    pub fn new_cont(cont: Cont) -> Self {
        Self::Cont(cont)
    }

    pub fn new_eol(eol: Eol) -> Self {
        Self::Eol(eol)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Step {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let (xi, fields) = data.as_constr().ok_or(anyhow!("Expect constr"))?;
        let fields = fields.collect::<Vec<PlutusData>>();
        if xi == 0 {
            let [a] = <[PlutusData; 1]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_cont(Cont::try_from(a)?))
        } else if xi == 1 {
            let [a] = <[PlutusData; 1]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_eol(Eol::try_from(a)?))
        } else {
            Err(anyhow!("Bad tag"))
        }
    }
}

impl<'a> From<Step> for PlutusData<'a> {
    fn from(value: Step) -> Self {
        match value {
            Step::Cont(x) => PlutusData::constr(0, vec![PlutusData::from(x)]),
            Step::Eol(x) => PlutusData::constr(1, vec![PlutusData::from(x)]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Cont {
    Add,
    Sub(Squash, Unlockeds),
    Close,
    Respond(Squash, MixedCheques),
    Unlock(Unpends),
    Expire(Unpends),
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

    pub fn new_respond(squash: Squash, mixed_cheques: MixedCheques) -> Self {
        Self::Respond(squash, mixed_cheques)
    }

    pub fn new_unlock(unpends: Unpends) -> Self {
        Self::Unlock(unpends)
    }

    pub fn new_expire(unpends: Unpends) -> Self {
        Self::Expire(unpends)
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Cont {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let (xi, fields) = data.as_constr().ok_or(anyhow!("Expect constr"))?;
        let fields = fields.collect::<Vec<PlutusData>>();
        if xi == 0 {
            let [] = <[PlutusData; 0]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_add())
        } else if xi == 1 {
            let [a, b] = <[PlutusData; 2]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_sub(
                Squash::try_from(&a)?,
                Unlockeds::try_from(&b)?,
            ))
        } else if xi == 2 {
            let [] = <[PlutusData; 0]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_close())
        } else if xi == 3 {
            let [a, b] = <[PlutusData; 2]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_respond(
                Squash::try_from(&a)?,
                MixedCheques::try_from(&b)?,
            ))
        } else if xi == 4 {
            let [a] = <[PlutusData; 1]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_unlock(Unpends::try_from(&a)?))
        } else if xi == 5 {
            let [a] = <[PlutusData; 1]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_expire(Unpends::try_from(&a)?))
        } else {
            Err(anyhow!("Bad tag"))
        }
    }
}

impl<'a> From<Cont> for PlutusData<'a> {
    fn from(value: Cont) -> Self {
        match value {
            Cont::Add => PlutusData::constr(0, vec![]),
            Cont::Sub(squash, unlockeds) => PlutusData::constr(
                0,
                vec![PlutusData::from(squash), PlutusData::from(unlockeds)],
            ),
            Cont::Close => PlutusData::constr(0, vec![]),
            Cont::Respond(squash, mixed_cheques) => PlutusData::constr(
                0,
                vec![PlutusData::from(squash), PlutusData::from(mixed_cheques)],
            ),
            Cont::Unlock(unpends) => PlutusData::constr(0, vec![PlutusData::from(unpends)]),
            Cont::Expire(unpends) => PlutusData::constr(0, vec![PlutusData::from(unpends)]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Eol {
    End,
    Elapse,
}

impl Eol {
    pub fn new_end() -> Self {
        Self::End
    }

    pub fn new_elapse() -> Self {
        Self::Elapse
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Eol {
    type Error = Error;

    fn try_from(data: PlutusData<'a>) -> Result<Self> {
        let (xi, fields) = data.as_constr().ok_or(anyhow!("Expect constr"))?;
        let fields = fields.collect::<Vec<PlutusData>>();
        if xi == 0 {
            let [] = <[PlutusData; 0]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_end())
        } else if xi == 1 {
            let [] = <[PlutusData; 0]>::try_from(fields).map_err(|_| anyhow!("Bad length"))?;
            Ok(Self::new_elapse())
        } else {
            Err(anyhow!("Bad tag"))
        }
    }
}

impl<'a> From<Eol> for PlutusData<'a> {
    fn from(value: Eol) -> Self {
        match value {
            Eol::End => PlutusData::constr(0, vec![]),
            Eol::Elapse => PlutusData::constr(1, vec![]),
        }
    }
}
