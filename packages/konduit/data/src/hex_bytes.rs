use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<T, S>(bytes: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<[u8]>,
    S: Serializer,
{
    let bytes = bytes.as_ref();
    if serializer.is_human_readable() {
        serializer.serialize_str(&hex::encode(bytes))
    } else {
        serializer.serialize_bytes(bytes)
    }
}

pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: TryFrom<Vec<u8>>,
    D: Deserializer<'de>,
{
    let bytes = if deserializer.is_human_readable() {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)?
    } else {
        Vec::<u8>::deserialize(deserializer)?
    };
    let len = bytes.len();
    T::try_from(bytes).map_err(|_| serde::de::Error::invalid_length(len, &"wrong length"))
}
