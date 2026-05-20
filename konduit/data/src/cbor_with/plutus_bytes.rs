use minicbor::{Decoder, Encoder};

const CHUNK_SIZE: usize = 64;

pub fn encode<C, W: minicbor::encode::Write>(
    val: &[u8],
    e: &mut Encoder<W>,
    _ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    if val.len() <= CHUNK_SIZE {
        e.bytes(val)?;
    } else {
        e.begin_bytes()?;
        for chunk in val.chunks(CHUNK_SIZE) {
            e.bytes(chunk)?;
        }
        e.end()?;
    }
    Ok(())
}

pub fn decode<'b, C>(
    d: &mut Decoder<'b>,
    _ctx: &mut C,
) -> Result<Vec<u8>, minicbor::decode::Error> {
    d.bytes_iter()?.try_fold(Vec::new(), |mut acc, chunk| {
        acc.extend_from_slice(chunk?);
        Ok(acc)
    })
}
