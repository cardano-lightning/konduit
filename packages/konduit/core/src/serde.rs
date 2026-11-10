//! Serde adapters for protocol byte values and byte containers.

use ::serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;

/// Serialize bytes as lowercase hex in human-readable formats and as a byte
/// string in binary formats. Supports `Vec<u8>` and fixed-size byte arrays.
pub mod bytes {
    use super::*;

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&hex::encode(value.as_ref()))
        } else {
            serializer.serialize_bytes(value.as_ref())
        }
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: TryFrom<Vec<u8>>,
        D: Deserializer<'de>,
    {
        let value = if deserializer.is_human_readable() {
            hex::decode(String::deserialize(deserializer)?).map_err(::serde::de::Error::custom)?
        } else {
            Vec::<u8>::deserialize(deserializer)?
        };
        let len = value.len();
        T::try_from(value).map_err(|_| ::serde::de::Error::invalid_length(len, &"byte value"))
    }
}

/// Adapter for `Option<T>` where `T` is a byte container.
pub mod option_bytes {
    use super::*;

    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&BytesRef(value)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: TryFrom<Vec<u8>>,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionBytes(PhantomData))
    }

    struct BytesRef<'a, T>(&'a T);

    impl<T: AsRef<[u8]>> Serialize for BytesRef<'_, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            super::bytes::serialize(self.0, serializer)
        }
    }

    struct OptionBytes<T>(PhantomData<T>);

    impl<'de, T> ::serde::de::Visitor<'de> for OptionBytes<T>
    where
        T: TryFrom<Vec<u8>>,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an optional hex string or byte string")
        }

        fn visit_none<E: ::serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: ::serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            super::bytes::deserialize(deserializer).map(Some)
        }
    }
}

/// Adapter for `Vec<T>` where each `T` is a byte container.
pub mod vec_bytes {
    use super::*;
    use ::serde::ser::SerializeSeq;

    pub fn serialize<T, S>(values: &[T], serializer: S) -> Result<S::Ok, S::Error>
    where
        T: AsRef<[u8]>,
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(values.len()))?;
        for value in values {
            seq.serialize_element(&BytesRef(value))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Vec<T>, D::Error>
    where
        T: TryFrom<Vec<u8>>,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BytesVec(PhantomData))
    }

    struct BytesRef<'a, T>(&'a T);

    impl<T: AsRef<[u8]>> Serialize for BytesRef<'_, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            super::bytes::serialize(self.0, serializer)
        }
    }

    struct BytesVec<T>(PhantomData<T>);

    impl<'de, T> ::serde::de::Visitor<'de> for BytesVec<T>
    where
        T: TryFrom<Vec<u8>>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a sequence of hex strings or byte strings")
        }

        fn visit_seq<A: ::serde::de::SeqAccess<'de>>(
            self,
            mut seq: A,
        ) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element_seed(BytesSeed::<T>(PhantomData))? {
                values.push(value);
            }
            Ok(values)
        }
    }

    struct BytesSeed<T>(PhantomData<T>);

    impl<'de, T> ::serde::de::DeserializeSeed<'de> for BytesSeed<T>
    where
        T: TryFrom<Vec<u8>>,
    {
        type Value = T;

        fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
            super::bytes::deserialize(deserializer)
        }
    }
}

/// Serialize any `Display`/`FromStr` type using its textual representation.
pub mod display_fromstr {
    use super::*;
    use std::{fmt::Display, str::FromStr};

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(::serde::de::Error::custom)
    }
}

/// Optional adapter for [`display_fromstr`].
pub mod option_display_fromstr {
    use super::*;
    use std::{fmt::Display, str::FromStr};

    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&DisplayRef(value)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionalDisplay(PhantomData))
    }

    struct DisplayRef<'a, T>(&'a T);

    impl<T: Display> Serialize for DisplayRef<'_, T> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&self.0.to_string())
        }
    }

    struct OptionalDisplay<T>(PhantomData<T>);

    impl<'de, T> ::serde::de::Visitor<'de> for OptionalDisplay<T>
    where
        T: FromStr,
        T::Err: Display,
    {
        type Value = Option<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an optional string-parsable value")
        }

        fn visit_none<E: ::serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: ::serde::de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            super::display_fromstr::deserialize(deserializer).map(Some)
        }
    }
}

#[cfg(all(test, feature = "json"))]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct ByteContainers {
        #[serde(with = "super::bytes")]
        fixed: [u8; 2],
        #[serde(with = "super::bytes")]
        bytes: Vec<u8>,
        #[serde(with = "super::option_bytes")]
        optional: Option<Vec<u8>>,
        #[serde(with = "super::vec_bytes")]
        chunks: Vec<Vec<u8>>,
    }

    #[test]
    fn byte_and_container_helpers_use_hex_json() {
        let value = ByteContainers {
            fixed: [0x01, 0xff],
            bytes: vec![0xaa, 0xbb],
            optional: Some(vec![0x10]),
            chunks: vec![vec![0x20], vec![0x30, 0x40]],
        };

        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            json,
            r#"{"fixed":"01ff","bytes":"aabb","optional":"10","chunks":["20","3040"]}"#
        );
        assert_eq!(
            serde_json::from_str::<ByteContainers>(&json).unwrap(),
            value
        );
    }

    #[test]
    fn optional_byte_helper_preserves_none() {
        let value = ByteContainers {
            fixed: [0, 0],
            bytes: Vec::new(),
            optional: None,
            chunks: Vec::new(),
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            serde_json::from_str::<ByteContainers>(&json).unwrap(),
            value
        );
    }
}
