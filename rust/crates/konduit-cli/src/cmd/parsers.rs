use cardano_sdk::{PlutusData, cbor::decode};
use konduit_data::{
    Cheque, ChequeBody, Keytag, Locked, Receipt, Secret, Squash, SquashBody, Unlocked,
};
use std::str::FromStr;

pub fn parse_squash(s: &str) -> anyhow::Result<Squash> {
    match s.split(",").collect::<Vec<_>>().as_slice() {
        [] => Err(anyhow::anyhow!("Cannot coerce from empty")),
        [x0] => Ok(Squash::try_from(PlutusData::from(hex::decode(x0)?))?),
        [x0, x1] => Ok(Squash::new(
            SquashBody::try_from(decode::<PlutusData>(&hex::decode(x0)?)?)?,
            x1.parse()?,
        )),
        _ => panic!("Not implemented error"),
    }
}

pub fn parse_locked(s: &str) -> anyhow::Result<Locked> {
    match s.split(",").collect::<Vec<_>>().as_slice() {
        [] => Err(anyhow::anyhow!("Cannot coerce from empty")),
        [x0] => Ok(Cheque::try_from(PlutusData::from(hex::decode(x0)?))?
            .as_locked()
            .ok_or(anyhow::anyhow!("Not a Locked"))?),
        [x0, x1] => Ok(Locked::new(
            ChequeBody::try_from(decode::<PlutusData>(&hex::decode(x0)?)?)?,
            x1.parse()?,
        )),
        _ => panic!("Not implemented error"),
    }
}

pub fn parse_cheque(s: &str) -> anyhow::Result<Cheque> {
    match s.split(",").collect::<Vec<_>>().as_slice() {
        [] => Err(anyhow::anyhow!("Cannot coerce from empty")),
        [x0] => Cheque::try_from(PlutusData::from(hex::decode(x0)?)),
        [x0, x1] => Ok(Cheque::from(Locked::new(
            ChequeBody::try_from(decode::<PlutusData>(&hex::decode(x0)?)?)?,
            x1.parse()?,
        ))),
        [x0, x1, x2] => Ok(Cheque::from(Unlocked::new(
            Locked::new(
                ChequeBody::try_from(decode::<PlutusData>(&hex::decode(x0)?)?)?,
                x1.parse()?,
            ),
            Secret::try_from(hex::decode(x2)?)?,
        )?)),
        _ => panic!("Not implemented error"),
    }
}

pub fn parse_keytag_receipt(s: &str) -> anyhow::Result<(Keytag, Receipt)> {
    let parts = s.split(";").collect::<Vec<_>>();
    let [x0, x1, x2 @ ..] = parts.as_slice() else {
        return Err(anyhow::anyhow!(
            "Must have at least keytag, squash, semicolon separated"
        ));
    };
    let keytag = Keytag::from_str(x0)?;
    let (key, tag) = keytag.split();
    let mut cheques = vec![];
    for x in x2 {
        let cheque = parse_cheque(x)?;
        if !cheque.verify(&key, &tag) {
            return Err(anyhow::anyhow!("Cheque not verified"));
        }
        cheques.push(cheque);
    }
    let receipt = Receipt::new_with_cheques(parse_squash(x1)?, cheques)?;
    Ok((keytag, receipt))
}
