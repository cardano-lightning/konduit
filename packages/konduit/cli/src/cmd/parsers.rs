use konduit_data::{Cheque, ChequeBody, Locked, Secret, Signature, Squash, SquashBody, Unlocked};

use konduit_tmp::{Keytag, Receipt};
use konduit_tx::to_verifying_key;
use std::str::FromStr;

pub fn parse_signature(x: &str) -> anyhow::Result<Signature> {
    let bytes = hex::decode(x)?;
    let arr = <[u8; 64]>::try_from(bytes)
        .map_err(|v| anyhow::anyhow!("Expected 64 bytes, got {}", v.len()))?;
    Ok(Signature::from(arr))
}

pub fn parse_squash(s: &str) -> anyhow::Result<Squash> {
    match s.split(",").collect::<Vec<_>>().as_slice() {
        [] => Err(anyhow::anyhow!("Cannot coerce from empty")),
        [x0] => Ok(minicbor::decode::<Squash>(&hex::decode(x0)?)?),
        [x0, x1] => Ok(Squash::new(
            minicbor::decode::<SquashBody>(&hex::decode(x0)?)?,
            parse_signature(x1)?,
        )),
        _ => panic!("Not implemented error"),
    }
}

pub fn parse_locked(s: &str) -> anyhow::Result<Locked> {
    match s.split(",").collect::<Vec<_>>().as_slice() {
        [] => Err(anyhow::anyhow!("Cannot coerce from empty")),
        [x0] => Ok(minicbor::decode::<Locked>(&hex::decode(x0)?)?),
        [x0, x1] => Ok(Locked::new(
            minicbor::decode::<ChequeBody>(&hex::decode(x0)?)?,
            parse_signature(x1)?,
        )),
        _ => panic!("Not implemented error"),
    }
}

pub fn parse_cheque(s: &str) -> anyhow::Result<Cheque> {
    match s.split(",").collect::<Vec<_>>().as_slice() {
        [] => Err(anyhow::anyhow!("Cannot coerce from empty")),
        [x0] => Ok(minicbor::decode::<Cheque>(&hex::decode(x0)?)?),
        [x0, x1] => Ok(Cheque::from(Locked::new(
            minicbor::decode::<ChequeBody>(&hex::decode(x0)?)?,
            parse_signature(x1)?,
        ))),
        [x0, x1, x2] => {
            let locked = Locked::new(
                minicbor::decode::<ChequeBody>(&hex::decode(x0)?)?,
                parse_signature(x1)?,
            );
            let secret = Secret::try_from(hex::decode(x2)?)?;
            let unlocked = Unlocked::try_from_locked(&locked, secret)
                .map_err(|_| anyhow::anyhow!("Bad secret"))?;
            Ok(Cheque::from(unlocked))
        }
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
    let key = to_verifying_key(key);
    let mut cheques = vec![];
    for x in x2 {
        let cheque = parse_cheque(x)?;
        let Ok(cheque) = cheque.try_verify(&key, &tag) else {
            return Err(anyhow::anyhow!("Cheque not verified"));
        };
        cheques.push(cheque);
    }
    let squash = parse_squash(x1)?.try_verify(&key, &tag)?;
    let receipt = Receipt::new_with_state(squash, cheques);
    Ok((keytag, receipt))
}
