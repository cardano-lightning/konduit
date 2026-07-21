use konduit_data::{Locked, Squash, SquashBody, Unlocked, Unverified, VerifyState};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode)]
#[serde(bound(deserialize = "V : Default"))]
pub struct SquashProposal<V: VerifyState = Unverified> {
    #[n(0)]
    pub proposal: SquashBody,
    #[n(1)]
    pub current: Squash<V>,
    #[n(2)]
    pub unlockeds: Vec<Unlocked<V>>,
    /// This is purely informational
    #[n(3)]
    pub lockeds: Vec<Locked<V>>,
}

impl<'b, C> Decode<'b, C> for SquashProposal<Unverified> {
    fn decode(d: &mut minicbor::Decoder<'b>, ctx: &mut C) -> Result<Self, minicbor::decode::Error> {
        d.array()?;
        let proposal: SquashBody = d.decode_with(ctx)?;
        let current: Squash<Unverified> = d.decode_with(ctx)?;
        let unlockeds: Vec<Unlocked<Unverified>> = d.decode_with(ctx)?;
        let lockeds: Vec<Locked<Unverified>> = d.decode_with(ctx)?;
        if d.datatype()? != minicbor::data::Type::Break {
            return Err(minicbor::decode::Error::message(
                "expected end of SquashProposal array",
            ));
        }
        d.skip()?;
        Ok(Self {
            proposal,
            current,
            unlockeds,
            lockeds,
        })
    }
}
