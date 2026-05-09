use crate::CborWith;

pub struct DisplayFromStr(());

impl<T, C> CborWith<T, C> for DisplayFromStr
where
    T: std::fmt::Display + std::str::FromStr,
    T::Err: std::fmt::Display,
{
    fn encode<W>(
        val: &T,
        e: &mut minicbor::Encoder<W>,
        _ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write,
    {
        e.str(&val.to_string())?;
        Ok(())
    }

    fn decode<'b>(
        d: &mut minicbor::Decoder<'b>,
        _ctx: &mut C,
    ) -> Result<T, minicbor::decode::Error> {
        d.str()?
            .parse()
            .map_err(|e: T::Err| minicbor::decode::Error::message(e.to_string()))
    }
}
