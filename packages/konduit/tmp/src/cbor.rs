pub fn encode_display<Ctx, T: core::fmt::Display, W: minicbor::encode::Write>(
    v: &T,
    e: &mut minicbor::Encoder<W>,
    _ctx: &mut Ctx,
) -> Result<(), minicbor::encode::Error<W::Error>> {
    e.str(&v.to_string())?;
    Ok(())
}

pub fn decode_from_str<'b, Ctx, T: core::str::FromStr>(
    d: &mut minicbor::Decoder<'b>,
    _ctx: &mut Ctx,
) -> Result<T, minicbor::decode::Error> {
    d.str()?
        .parse()
        .map_err(|_| minicbor::decode::Error::message("invalid string"))
}
