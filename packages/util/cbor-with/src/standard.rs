#[cfg(any(feature = "crypto", feature = "std"))]
use crate::as_module;

#[cfg(feature = "crypto")]
use crate::adapters::FixedBytes;
#[cfg(feature = "std")]
use crate::adapters::{Bytes, BytesOwned, DisplayFromStr, Same};
#[cfg(feature = "std")]
use crate::composers::{AsNullable, AsOptional, AsVec};

#[cfg(feature = "std")]
as_module!(pub mod same = Same);
#[cfg(feature = "std")]
as_module!(pub mod display_from_str = DisplayFromStr);
#[cfg(feature = "std")]
as_module!(pub mod bytes = Bytes);
#[cfg(feature = "std")]
as_module!(pub mod bytes_owned = BytesOwned);

#[cfg(feature = "std")]
as_module!(pub mod nullable_same = AsNullable<Same>);
#[cfg(feature = "std")]
as_module!(pub mod nullable_display_from_str = AsNullable<DisplayFromStr>);
#[cfg(feature = "std")]
as_module!(pub mod nullable_bytes = AsNullable<Bytes>);
#[cfg(feature = "std")]
as_module!(pub mod nullable_bytes_owned = AsNullable<BytesOwned>);

#[cfg(feature = "std")]
as_module!(pub mod optional_same = AsOptional<Same>);
#[cfg(feature = "std")]
as_module!(pub mod optional_display_from_str = AsOptional<DisplayFromStr>);
#[cfg(feature = "std")]
as_module!(pub mod optional_bytes = AsOptional<Bytes>);
#[cfg(feature = "std")]
as_module!(pub mod optional_bytes_owned = AsOptional<BytesOwned>);

#[cfg(feature = "std")]
as_module!(pub mod vec_same = AsVec<Same>);
#[cfg(feature = "std")]
as_module!(pub mod vec_display_from_str = AsVec<DisplayFromStr>);
#[cfg(feature = "std")]
as_module!(pub mod vec_bytes = AsVec<Bytes>);
#[cfg(feature = "std")]
as_module!(pub mod vec_bytes_owned = AsVec<BytesOwned>);

#[cfg(feature = "crypto")]
as_module!(pub mod fixed_bytes_32 = FixedBytes<32>);
#[cfg(feature = "crypto")]
as_module!(pub mod fixed_bytes_64 = FixedBytes<64>);
#[cfg(feature = "crypto")]
as_module!(pub mod nullable_fixed_bytes_32 = AsNullable<FixedBytes<32>>);
#[cfg(feature = "crypto")]
as_module!(pub mod nullable_fixed_bytes_64 = AsNullable<FixedBytes<64>>);
#[cfg(feature = "crypto")]
as_module!(pub mod optional_fixed_bytes_32 = AsOptional<FixedBytes<32>>);
#[cfg(feature = "crypto")]
as_module!(pub mod optional_fixed_bytes_64 = AsOptional<FixedBytes<64>>);
#[cfg(feature = "crypto")]
as_module!(pub mod vec_fixed_bytes_32 = AsVec<FixedBytes<32>>);
#[cfg(feature = "crypto")]
as_module!(pub mod vec_fixed_bytes_64 = AsVec<FixedBytes<64>>);
