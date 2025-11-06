pub use hex;
pub use serde;

// use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// This public module holds the helper trait that allows the macro
/// to be generic over the inner container type (Vec<u8> or [u8; N]).
pub trait HexBytes: Sized {
    fn from_vec(v: ::std::vec::Vec<u8>) -> Result<Self, ::std::string::String>;
}

impl HexBytes for ::std::vec::Vec<u8> {
    fn from_vec(v: ::std::vec::Vec<u8>) -> Result<Self, ::std::string::String> {
        Ok(v)
    }
}

impl<const N: usize> HexBytes for [u8; N] {
    fn from_vec(v: ::std::vec::Vec<u8>) -> Result<Self, ::std::string::String> {
        v.try_into()
            .map_err(|v: Vec<u8>| ::std::format!("invalid length: expected {}, got {}", N, v.len()))
    }
}

/// Macro to implement hex-based Serde for a wrapper type
/// that holds a single `Vec<u8>` or `[u8; N]`.
#[macro_export]
macro_rules! impl_hex_serde_for_wrapper {
    ($wrapper_type:ident, $inner_type:ty) => {
        impl Serialize for $wrapper_type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: $crate::serde::Serializer,
            {
                let hex_string = $crate::hex::encode(&self.0);
                serializer.serialize_str(&hex_string)
            }
        }

        impl<'de> Deserialize<'de> for $wrapper_type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: $crate::serde::Deserializer<'de>,
            {
                struct HexVisitor;

                impl<'de> $crate::serde::de::Visitor<'de> for HexVisitor {
                    type Value = $wrapper_type;

                    fn expecting(
                        &self,
                        formatter: &mut ::std::fmt::Formatter,
                    ) -> ::std::fmt::Result {
                        formatter.write_str("a hex-encoded string")
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: $crate::serde::de::Error,
                    {
                        let bytes = match $crate::hex::decode(v) {
                            Ok(bytes) => bytes,
                            Err(e) => {
                                return Err($crate::serde::de::Error::custom(::std::format!(
                                    "failed to decode hex string: {}",
                                    e
                                )));
                            }
                        };

                        let inner_val = <$inner_type as $crate::HexBytes>::from_vec(bytes)
                            .map_err($crate::serde::de::Error::custom)?;

                        Ok($wrapper_type(inner_val))
                    }
                }

                deserializer.deserialize_str(HexVisitor)
            }
        }
    };
}
