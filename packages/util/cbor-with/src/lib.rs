pub mod prelude;

mod adapters;
pub use adapters::*;

mod composers;
pub use composers::*;

pub trait CborWith<T, C = ()> {
    fn encode<W>(
        val: &T,
        e: &mut minicbor::Encoder<W>,
        ctx: &mut C,
    ) -> Result<(), minicbor::encode::Error<W::Error>>
    where
        W: minicbor::encode::Write;

    fn decode<'b>(d: &mut minicbor::Decoder<'b>, ctx: &mut C)
    -> Result<T, minicbor::decode::Error>;
}

mod standard;
#[cfg(any(feature = "crypto", feature = "std"))]
pub use standard::*;
