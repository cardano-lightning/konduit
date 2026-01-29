use crate::{Cheque, Squash, Unlocked, Unpend};
use anyhow::anyhow;
use cardano_tx_builder::{PlutusData, constr};

#[derive(Debug, Clone)]
pub enum Redeemer {
    Defer,
    Main(Vec<Step>),
    Mutual,
}

impl<'a> TryFrom<&PlutusData<'a>> for Redeemer {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
        match tag {
            0 => Ok(Redeemer::Defer),
            1 => {
                let [steps] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                    .map_err(|_| anyhow!("invalid 'Cheque'"))?;
                let steps = <Vec<PlutusData>>::try_from(&steps)?
                    .into_iter()
                    .map(Step::try_from)
                    .collect::<anyhow::Result<Vec<Step>>>()?;
                Ok(Redeemer::Main(steps))
            }
            2 => Ok(Redeemer::Mutual),
            _ => Err(anyhow!("Unknown Redeemer tag: {}", tag)),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Redeemer {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Redeemer::try_from(&data)
    }
}

impl<'a> From<Redeemer> for PlutusData<'a> {
    fn from(value: Redeemer) -> Self {
        match value {
            Redeemer::Defer => constr!(0),
            Redeemer::Main(steps) => constr!(
                1,
                PlutusData::list(
                    steps
                        .into_iter()
                        .map(PlutusData::from)
                        .collect::<Vec<PlutusData>>(),
                ),
            ),
            Redeemer::Mutual => constr!(2),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Step {
    Cont(Cont),
    Eol(Eol),
}

impl<'a> TryFrom<&PlutusData<'a>> for Step {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
        let [field] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
            .map_err(|_| anyhow!("invalid 'Step'"))?;
        match tag {
            0 => {
                let cont = Cont::try_from(field)?;
                Ok(Step::Cont(cont))
            }
            1 => {
                let eol = Eol::try_from(field)?;
                Ok(Step::Eol(eol))
            }
            _ => Err(anyhow!("Unknown Step tag: {}", tag)),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Step {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Step::try_from(&data)
    }
}

impl<'a> From<Step> for PlutusData<'a> {
    fn from(value: Step) -> Self {
        match value {
            Step::Cont(cont) => constr!(0, PlutusData::from(cont)),
            Step::Eol(eol) => constr!(1, PlutusData::from(eol)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Cont {
    Add,
    Sub(Squash, Vec<Unlocked>),
    Close,
    Respond(Squash, Vec<Cheque>),
    Unlock(Vec<Unpend>),
    Expire(Vec<Unpend>),
}

impl<'a> TryFrom<&PlutusData<'a>> for Cont {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let (tag, fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
        match tag {
            0 => Ok(Cont::Add),
            1 => {
                let [a, b] = <[PlutusData; 2]>::try_from(fields.collect::<Vec<_>>())
                    .map_err(|_| anyhow!("invalid 'Cont::Sub'"))?;
                let squash = Squash::try_from(&a)?;
                let unlocked = <Vec<PlutusData>>::try_from(&b)?
                    .into_iter()
                    .map(|x| Unlocked::try_from(&x))
                    .collect::<anyhow::Result<Vec<Unlocked>>>()?;
                Ok(Cont::Sub(squash, unlocked))
            }
            2 => Ok(Cont::Close),
            3 => {
                let [a, b] = <[PlutusData; 2]>::try_from(fields.collect::<Vec<_>>())
                    .map_err(|_| anyhow!("invalid 'Cont::Sub'"))?;
                let squash = Squash::try_from(&a)?;
                let cheques = <Vec<PlutusData>>::try_from(&b)?
                    .into_iter()
                    .map(Cheque::try_from)
                    .collect::<anyhow::Result<Vec<Cheque>>>()?;
                Ok(Cont::Respond(squash, cheques))
            }
            4 => {
                let [a] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                    .map_err(|_| anyhow!("invalid 'Cont::Unlock'"))?;
                let unpends = <Vec<PlutusData>>::try_from(&a)?
                    .into_iter()
                    .map(|x| {
                        let bytes = Unpend::try_from(x)?;
                        Ok(bytes)
                    })
                    .collect::<anyhow::Result<Vec<Unpend>>>()?;
                Ok(Cont::Unlock(unpends))
            }
            5 => {
                let [field] = <[PlutusData; 1]>::try_from(fields.collect::<Vec<_>>())
                    .map_err(|_| anyhow!("invalid 'Cont::Expire'"))?;
                let unpends = <Vec<PlutusData>>::try_from(&field)?
                    .into_iter()
                    .map(|x| {
                        let bytes = <Unpend>::try_from(x)?;
                        Ok(bytes)
                    })
                    .collect::<anyhow::Result<Vec<Unpend>>>()?;
                Ok(Cont::Expire(unpends))
            }
            _ => Err(anyhow!("Unknown Cont tag: {}", tag)),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Cont {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Cont::try_from(&data)
    }
}

impl<'a> From<Cont> for PlutusData<'a> {
    fn from(value: Cont) -> Self {
        match value {
            Cont::Add => constr!(0),
            Cont::Sub(squash, unlocked) => constr!(
                1,
                PlutusData::from(squash),
                PlutusData::list(
                    unlocked
                        .into_iter()
                        .map(PlutusData::from)
                        .collect::<Vec<PlutusData>>(),
                )
            ),
            Cont::Close => constr!(2),
            Cont::Respond(squash, cheques) => constr!(
                3,
                PlutusData::from(squash),
                PlutusData::list(
                    cheques
                        .into_iter()
                        .map(PlutusData::from)
                        .collect::<Vec<PlutusData>>(),
                ),
            ),
            Cont::Unlock(unpends) => constr!(
                4,
                PlutusData::list(
                    unpends
                        .into_iter()
                        .map(PlutusData::from)
                        .collect::<Vec<PlutusData>>(),
                ),
            ),
            Cont::Expire(unpends) => constr!(
                5,
                PlutusData::list(
                    unpends
                        .into_iter()
                        .map(PlutusData::from)
                        .collect::<Vec<PlutusData>>(),
                ),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Eol {
    End,
    Elapse,
}

impl<'a> TryFrom<&PlutusData<'a>> for Eol {
    type Error = anyhow::Error;

    fn try_from(data: &PlutusData<'a>) -> anyhow::Result<Self> {
        let (tag, _fields) = data.as_constr().ok_or(anyhow!("Not a constructor"))?;
        match tag {
            0 => Ok(Eol::End),
            1 => Ok(Eol::Elapse),
            _ => Err(anyhow!("Unknown Eol tag: {}", tag)),
        }
    }
}

impl<'a> TryFrom<PlutusData<'a>> for Eol {
    type Error = anyhow::Error;

    fn try_from(data: PlutusData<'a>) -> anyhow::Result<Self> {
        Eol::try_from(&data)
    }
}

impl<'a> From<Eol> for PlutusData<'a> {
    fn from(value: Eol) -> Self {
        match value {
            Eol::End => constr!(0),
            Eol::Elapse => constr!(1),
        }
    }
}
