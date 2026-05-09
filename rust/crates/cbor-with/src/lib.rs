// pub mod display_from_str;
// pub mod via_array;
// pub mod option;
//
// pub use codec::{CborWith, AsOption, AsVec, DisplayFromStr};
//
// #[cfg(feature = "plutus-data")]
// pub mod via_plutus_data;

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

// Pre-built modules for common cases
as_module!(pub mod display_from_str       = DisplayFromStr);
as_module!(pub mod option_display_from_str   = AsOption<DisplayFromStr>);
as_module!(pub mod vec_display_from_str   = AsVec<DisplayFromStr>);
