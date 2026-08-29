use cardano_sdk::VerificationKey;
use konduit_data::Duration;
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// These variables are either those used by more than one component,
/// or are mandatory.
/// These are not all variables required: component specific ones
/// are colocated with the component.
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ChannelParameters {
    #[serde_as(as = "serde_with::hex::Hex")]
    #[cbor(n(0), encode_with = "encode_vk", decode_with = "decode_vk")]
    pub adaptor_key: VerificationKey,
    #[n(1)]
    pub close_period: Duration,
    #[n(2)]
    pub tag_length: usize,
}

// FIXME :: upstream this.
fn encode_vk<Ctx, W: minicbor::encode::Write>(
    v: &VerificationKey,
    e: &mut minicbor::Encoder<W>,
    _ctx: &mut Ctx,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    let bytes: [u8; 32] = (*v).into();
    e.bytes(&bytes)?;
    Ok(())
}

fn decode_vk<'b, Ctx>(
    d: &mut minicbor::Decoder<'b>,
    _ctx: &mut Ctx,
) -> Result<VerificationKey, minicbor::decode::Error> {
    let bytes: [u8; 32] = d
        .bytes()?
        .try_into()
        .map_err(|_| minicbor::decode::Error::message("expected 32 bytes"))?;
    Ok(bytes.into())
}
