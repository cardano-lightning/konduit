use minicbor::{Decoder, Encoder};

pub fn encode<C, W, T>(
    val: &Vec<T>,
    e: &mut Encoder<W>,
    ctx: &mut C,
) -> Result<(), minicbor::encode::Error<W::Error>>
where
    W: minicbor::encode::Write,
    T: minicbor::Encode<C>,
{
    if val.is_empty() {
        e.array(0)?;
    } else {
        e.begin_array()?;
        for item in val {
            e.encode_with(item, ctx)?;
        }
        e.end()?;
    }
    Ok(())
}

pub fn decode<'b, C, T>(d: &mut Decoder<'b>, ctx: &mut C) -> Result<Vec<T>, minicbor::decode::Error>
where
    T: minicbor::Decode<'b, C>,
{
    let mut result = Vec::new();
    for item in d.array_iter_with(ctx)? {
        result.push(item?);
    }
    Ok(result)
}
