//! Type definitions shared between Kupo endpoints.
//!
//! These mirror the JSON shapes described in the [Kupo API reference](https://cardanosolutions.github.io/kupo/).

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde_with::serde_as;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseBytesError {
    #[error("invalid base16: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("invalid byte length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("invalid byte length for {name}: expected {min}..={max}, got {actual}")]
    InvalidLengthRange {
        name: &'static str,
        min: usize,
        max: usize,
        actual: usize,
    },
}

macro_rules! fixed_bytes_type {
    ($name:ident, $size:literal, $doc:literal) => {
        #[doc = $doc]
        #[serde_as]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            serde::Deserialize,
            serde::Serialize,
        )]
        #[serde(transparent)]
        pub struct $name(#[serde_as(as = "serde_with::hex::Hex")] pub [u8; $size]);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&hex::encode(self.0))
            }
        }

        impl FromStr for $name {
            type Err = ParseBytesError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let bytes = hex::decode(value)?;
                Self::try_from(bytes)
            }
        }

        impl TryFrom<Vec<u8>> for $name {
            type Error = ParseBytesError;

            fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
                let actual = bytes.len();
                let bytes = bytes
                    .try_into()
                    .map_err(|_| ParseBytesError::InvalidLength {
                        expected: $size,
                        actual,
                    })?;
                Ok(Self(bytes))
            }
        }

        impl From<[u8; $size]> for $name {
            fn from(bytes: [u8; $size]) -> Self {
                Self(bytes)
            }
        }

        impl AsRef<[u8; $size]> for $name {
            fn as_ref(&self) -> &[u8; $size] {
                &self.0
            }
        }
    };
}

fixed_bytes_type!(
    Blake2b224,
    28,
    "A 28-byte blake2b-224 hash digest used for script and policy hashes."
);
fixed_bytes_type!(
    Blake2b256,
    32,
    "A 32-byte blake2b-256 hash digest used for transaction, datum, and header hashes."
);

macro_rules! bounded_bytes_type {
    ($name:ident, $min:literal, $max:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Vec<u8>);

        impl $name {
            pub fn into_bytes(self) -> Vec<u8> {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&hex::encode(&self.0))
            }
        }

        impl FromStr for $name {
            type Err = ParseBytesError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::try_from(hex::decode(value)?)
            }
        }

        impl TryFrom<Vec<u8>> for $name {
            type Error = ParseBytesError;

            fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
                let actual = bytes.len();
                if !($min..=$max).contains(&actual) {
                    return Err(ParseBytesError::InvalidLengthRange {
                        name: stringify!($name),
                        min: $min,
                        max: $max,
                        actual,
                    });
                }
                Ok(Self(bytes))
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

bounded_bytes_type!(
    AssetName,
    0,
    32,
    "A base16-encoded Cardano asset name containing 0 to 32 bytes."
);

bounded_bytes_type!(
    MetadataBytes,
    0,
    64,
    "A metadata byte string containing at most 64 bytes."
);

/// A reference to a block on the chain: slot number + header hash.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Point {
    pub slot_no: u64,
    pub header_hash: Blake2b256,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Checkpoint {
    pub block_no: u64,
    pub slot_no: u64,
    pub header_hash: Blake2b256,
}

/// A (multi-asset) value of a transaction output.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub struct Value {
    pub coins: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<AssetId, u64>,
}

/// A resolved Plutus' datum.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Datum {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub datum: Vec<u8>,
}

/// The type of datum referenced in an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatumType {
    Hash,
    Inline,
}

/// A resolved native or Plutus script.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Script {
    pub language: ScriptLanguage,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub script: Vec<u8>,
}

/// The type of script returned by the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum ScriptLanguage {
    #[serde(rename = "native")]
    Native,
    #[serde(rename = "plutus:v1")]
    PlutusV1,
    #[serde(rename = "plutus:v2")]
    PlutusV2,
    #[serde(rename = "plutus:v3")]
    PlutusV3,
}

impl ScriptLanguage {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::PlutusV1 => "plutus:v1",
            Self::PlutusV2 => "plutus:v2",
            Self::PlutusV3 => "plutus:v3",
        }
    }
}

/// A point at which an input was spent.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct SpentAt {
    pub block_no: u64,
    pub slot_no: u64,
    pub header_hash: Blake2b256,
    pub transaction_id: Blake2b256,
    pub input_index: u16,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub redeemer: Vec<u8>,
    pub transaction_index: u64,
}

/// A single match: a transaction output matched by a pattern.
#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Match {
    pub address: String,
    pub created_at: Checkpoint,
    #[serde_as(as = "Option<serde_with::hex::Hex>")]
    pub datum: Option<Vec<u8>>,
    pub datum_hash: Option<Blake2b256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datum_type: Option<DatumType>,
    pub output_index: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script_hash: Option<Blake2b224>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<Script>,
    pub spent_at: Option<SpentAt>,
    pub transaction_index: u64,
    pub transaction_id: Blake2b256,
    pub value: Value,
}

/// A validated Cardano asset identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetId {
    pub policy_id: Blake2b224,
    pub asset_name: Option<AssetName>,
}

impl AssetId {
    pub fn policy(policy_id: Blake2b224) -> Self {
        Self {
            policy_id,
            asset_name: None,
        }
    }

    pub fn asset(policy_id: Blake2b224, asset_name: AssetName) -> Self {
        Self {
            policy_id,
            asset_name: Some(asset_name),
        }
    }
}

impl fmt::Display for AssetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.policy_id)?;
        if let Some(asset_name) = &self.asset_name {
            write!(f, ".{asset_name}")?;
        }
        Ok(())
    }
}

impl FromStr for AssetId {
    type Err = ParseBytesError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (policy_id, asset_name) = match value.split_once('.') {
            Some((policy_id, asset_name)) => (policy_id.parse()?, Some(asset_name.parse()?)),
            None => (value.parse()?, None),
        };
        Ok(Self {
            policy_id,
            asset_name,
        })
    }
}

impl serde::Serialize for AssetId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for AssetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// A matching pattern on addresses, assets or transactions.
///
/// See the [Patterns section of the Kupo API](https://cardanosolutions.github.io/kupo/#section/Patterns)
/// for details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pattern {
    /// The wildcard pattern, matching all addresses.
    Wildcard,
    /// An address pattern (or stake address), in any of the formats accepted by Kupo.
    Address(String),
    /// An asset id pattern: a policy id and an optional asset name.
    AssetId {
        policy_id: Blake2b224,
        asset_name: AssetNamePattern,
    },
    /// An output reference pattern: an optional output index and a transaction id.
    OutputReference {
        index: Option<u64>,
        transaction_id: Blake2b256,
    },
}

/// The asset name part of an [`Pattern::AssetId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetNamePattern {
    /// Match any asset name under the given policy.
    Wildcard,
    /// Match a specific asset name.
    Name(AssetName),
}

impl Pattern {
    /// The wildcard pattern, matching all addresses.
    pub fn wildcard() -> Self {
        Self::Wildcard
    }

    /// An address or stake address pattern.
    pub fn address(address: impl Into<String>) -> Self {
        Self::Address(address.into())
    }

    /// An asset id pattern matching a specific policy and any asset name.
    pub fn policy(policy_id: Blake2b224) -> Self {
        Self::AssetId {
            policy_id,
            asset_name: AssetNamePattern::Wildcard,
        }
    }

    /// An asset id pattern matching a specific policy and asset name.
    pub fn asset(policy_id: Blake2b224, asset_name: AssetName) -> Self {
        Self::AssetId {
            policy_id,
            asset_name: AssetNamePattern::Name(asset_name),
        }
    }

    /// An output reference pattern matching a specific transaction id, any output index.
    pub fn tx(transaction_id: Blake2b256) -> Self {
        Self::OutputReference {
            index: None,
            transaction_id,
        }
    }

    /// An output reference pattern matching a specific transaction id and output index.
    pub fn output_ref(index: u64, transaction_id: Blake2b256) -> Self {
        Self::OutputReference {
            index: Some(index),
            transaction_id,
        }
    }
}

impl FromStr for Pattern {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "*" {
            return Ok(Self::Wildcard);
        }

        if let Some((index, transaction_id)) = value.split_once('@') {
            let index = match index {
                "*" => None,
                value => Some(
                    value
                        .parse()
                        .map_err(|error| format!("invalid output index: {error}"))?,
                ),
            };
            let transaction_id = transaction_id
                .parse()
                .map_err(|error| format!("invalid transaction id: {error}"))?;
            return Ok(Self::OutputReference {
                index,
                transaction_id,
            });
        }

        if let Some((policy_id, asset_name)) = value.split_once('.') {
            let policy_id = policy_id
                .parse()
                .map_err(|error| format!("invalid policy id: {error}"))?;
            let asset_name = match asset_name {
                "*" => AssetNamePattern::Wildcard,
                value => AssetNamePattern::Name(
                    value
                        .parse()
                        .map_err(|error| format!("invalid asset name: {error}"))?,
                ),
            };
            return Ok(Self::AssetId {
                policy_id,
                asset_name,
            });
        }

        Ok(Self::Address(value.to_owned()))
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wildcard => f.write_str("*"),
            Self::Address(addr) => f.write_str(addr),
            Self::AssetId {
                policy_id,
                asset_name,
            } => {
                write!(f, "{policy_id}.")?;
                match asset_name {
                    AssetNamePattern::Wildcard => f.write_str("*"),
                    AssetNamePattern::Name(name) => write!(f, "{name}"),
                }
            }
            Self::OutputReference {
                index,
                transaction_id,
            } => {
                match index {
                    Some(i) => write!(f, "{}@", i)?,
                    None => f.write_str("*@")?,
                }
                write!(f, "{transaction_id}")
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

impl serde::Serialize for Pattern {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

/// Order results returned by the `/matches` endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Order {
    #[default]
    MostRecentFirst,
    OldestFirst,
}

impl Order {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MostRecentFirst => "most_recent_first",
            Self::OldestFirst => "oldest_first",
        }
    }
}

/// The status of matches: any (default), only spent, or only unspent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MatchStatus {
    #[default]
    Any,
    Spent,
    Unspent,
}

/// A lower or upper bound on the slot (or point) at which a match was created or spent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchBound {
    Slot(u64),
    Point {
        slot_no: u64,
        header_hash: Blake2b256,
    },
}

impl fmt::Display for MatchBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Slot(slot) => write!(f, "{}", slot),
            Self::Point {
                slot_no,
                header_hash,
            } => write!(f, "{}.{}", slot_no, header_hash),
        }
    }
}

/// Optional filters applicable to the `/matches` endpoints.
#[derive(Debug, Clone, Default)]
pub struct MatchFilters {
    pub resolve_hashes: bool,
    pub status: MatchStatus,
    pub order: Order,
    pub created_after: Option<MatchBound>,
    pub spent_after: Option<MatchBound>,
    pub created_before: Option<MatchBound>,
    pub spent_before: Option<MatchBound>,
    pub policy_id: Option<Blake2b224>,
    pub asset_name: Option<AssetName>,
    pub transaction_id: Option<Blake2b256>,
    pub output_index: Option<u64>,
}

impl MatchFilters {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to resolve and inline datum/script references in matches.
    pub fn with_resolve_hashes(mut self, resolve: bool) -> Self {
        self.resolve_hashes = resolve;
        self
    }

    /// Filter matches by status.
    pub fn with_status(mut self, status: MatchStatus) -> Self {
        self.status = status;
        self
    }

    /// Order matches by `most_recent_first` (default) or `oldest_first`.
    pub fn with_order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    /// Lower bound on the slot/point at which a match was created.
    pub fn with_created_after(mut self, bound: MatchBound) -> Self {
        self.created_after = Some(bound);
        self
    }

    /// Lower bound on the slot/point at which a match was spent.
    pub fn with_spent_after(mut self, bound: MatchBound) -> Self {
        self.spent_after = Some(bound);
        self
    }

    /// Upper bound on the slot/point at which a match was created.
    pub fn with_created_before(mut self, bound: MatchBound) -> Self {
        self.created_before = Some(bound);
        self
    }

    /// Upper bound on the slot/point at which a match was spent.
    pub fn with_spent_before(mut self, bound: MatchBound) -> Self {
        self.spent_before = Some(bound);
        self
    }

    /// Restrict matches to outputs containing an asset under the given policy.
    pub fn with_policy(mut self, policy_id: Blake2b224) -> Self {
        self.policy_id = Some(policy_id);
        self
    }

    /// Restrict matches to outputs containing the given asset name. Must be combined with a `policy_id`.
    pub fn with_asset_name(mut self, name: AssetName) -> Self {
        self.asset_name = Some(name);
        self
    }

    /// Restrict matches to outputs at the given transaction id.
    pub fn with_transaction(mut self, tx_id: Blake2b256) -> Self {
        self.transaction_id = Some(tx_id);
        self
    }

    /// Restrict matches to the given output index. Must be combined with a `transaction_id`.
    pub fn with_output_index(mut self, index: u64) -> Self {
        self.output_index = Some(index);
        self
    }
}

/// A point to rollback the synchronization to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackTo {
    pub slot_no: u64,
    pub header_hash: Option<Blake2b256>,
}

impl RollbackTo {
    pub fn slot(slot_no: u64) -> Self {
        Self {
            slot_no,
            header_hash: None,
        }
    }

    pub fn point(slot_no: u64, header_hash: Blake2b256) -> Self {
        Self {
            slot_no,
            header_hash: Some(header_hash),
        }
    }
}

/// Behavior of a rollback when reaching outside the safe zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Limit {
    #[default]
    WithinSafeZone,
    UnsafeAllowBeyondSafeZone,
}

impl Limit {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WithinSafeZone => "within_safe_zone",
            Self::UnsafeAllowBeyondSafeZone => "unsafe_allow_beyond_safe_zone",
        }
    }
}

/// A forced rollback to apply when adding patterns.
#[derive(Debug, Clone)]
pub struct ForcedRollback {
    pub rollback_to: RollbackTo,
    pub limit: Limit,
}

impl ForcedRollback {
    pub fn to(rollback_to: RollbackTo) -> Self {
        Self {
            rollback_to,
            limit: Limit::default(),
        }
    }

    pub fn with_limit(mut self, limit: Limit) -> Self {
        self.limit = limit;
        self
    }
}

/// Response body for delete endpoints, reporting the number of entities removed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Deleted {
    pub deleted: u64,
}

/// Body for the `/patterns` PUT endpoint describing a forced rollback.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct ForcedRollbackBody<'a> {
    pub rollback_to: RollbackToBody<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<&'a str>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct RollbackToBody<'a> {
    pub slot_no: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_hash: Option<&'a Blake2b256>,
}

impl<'a> From<&'a ForcedRollback> for ForcedRollbackBody<'a> {
    fn from(rollback: &'a ForcedRollback) -> Self {
        Self {
            rollback_to: RollbackToBody {
                slot_no: rollback.rollback_to.slot_no,
                header_hash: rollback.rollback_to.header_hash.as_ref(),
            },
            limit: Some(rollback.limit.as_str()),
        }
    }
}

/// Body for the `/patterns` bulk PUT endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct BulkPatternsBody<'a> {
    pub patterns: &'a [Pattern],
    #[serde(flatten)]
    pub forced_rollback: ForcedRollbackBody<'a>,
}

/// Body for a single-pattern PUT endpoint.
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct PutPatternBody<'a> {
    #[serde(flatten)]
    pub forced_rollback: ForcedRollbackBody<'a>,
}

/// An error response from Kupo.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct BadRequest {
    pub hint: Option<String>,
}

/// An overview of the server & connection status.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Health {
    pub connection_status: ConnectionStatus,
    pub most_recent_checkpoint: Option<u64>,
    pub most_recent_node_tip: Option<u64>,
    pub seconds_since_last_block: Option<u64>,
    pub network_synchronization: Option<f64>,
    pub configuration: HealthConfiguration,
    pub version: String,
}

/// Connection status with the underlying node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
}

/// A subset of the server's configuration reported through `/health`.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct HealthConfiguration {
    pub indexes: HealthIndexes,
}

/// Behavior surrounding database query indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthIndexes {
    Deferred,
    Installed,
}

/// A metadata blob associated with a transaction.
#[serde_as]
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    pub hash: Blake2b256,
    pub schema: Metadatum,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub raw: Vec<u8>,
}

/// A high-level description of a metadata value.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(untagged)]
pub enum Metadatum {
    Int(MetadatumInt),
    String(MetadatumString),
    Bytes(MetadatumBytes),
    List(MetadatumList),
    Map(MetadatumMap),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetadatumInt {
    pub int: serde_json::Value,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetadatumString {
    pub string: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetadatumBytes {
    pub bytes: MetadataBytes,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetadatumList {
    pub list: Vec<Metadatum>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetadatumMap {
    pub map: Vec<MetadatumEntry>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct MetadatumEntry {
    pub k: Box<Metadatum>,
    pub v: Box<Metadatum>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_hashes_roundtrip_as_base16() {
        let hash28 = Blake2b224([0xab; 28]);
        let hash32 = Blake2b256([0xcd; 32]);

        let encoded28 = serde_json::to_string(&hash28).unwrap();
        let encoded32 = serde_json::to_string(&hash32).unwrap();

        assert_eq!(encoded28, format!("\"{}\"", "ab".repeat(28)));
        assert_eq!(encoded32, format!("\"{}\"", "cd".repeat(32)));
        assert_eq!(
            serde_json::from_str::<Blake2b224>(&encoded28).unwrap(),
            hash28
        );
        assert_eq!(
            serde_json::from_str::<Blake2b256>(&encoded32).unwrap(),
            hash32
        );
    }

    #[test]
    fn fixed_hashes_reject_malformed_base16_and_wrong_lengths() {
        assert!(serde_json::from_str::<Blake2b224>("\"not-hex\"").is_err());
        assert!(serde_json::from_str::<Blake2b224>("\"ab\"").is_err());
        assert!(serde_json::from_str::<Blake2b256>("\"abc\"").is_err());
        assert!(Blake2b256::try_from(vec![0; 31]).is_err());
        assert!(Blake2b256::try_from(vec![0; 33]).is_err());
    }

    #[test]
    fn variable_binary_data_roundtrips_as_base16() {
        let datum = Datum {
            datum: vec![0xd8, 0x79, 0x80],
        };
        let script = Script {
            language: ScriptLanguage::PlutusV3,
            script: vec![0x01, 0x02, 0xff],
        };

        let datum_json = serde_json::to_string(&datum).unwrap();
        let script_json = serde_json::to_string(&script).unwrap();

        assert_eq!(datum_json, r#"{"datum":"d87980"}"#);
        assert_eq!(script_json, r#"{"language":"plutus:v3","script":"0102ff"}"#);
        assert_eq!(serde_json::from_str::<Datum>(&datum_json).unwrap(), datum);
        assert_eq!(
            serde_json::from_str::<Script>(&script_json).unwrap(),
            script
        );
        assert!(serde_json::from_str::<Datum>(r#"{"datum":"xyz"}"#).is_err());
    }

    #[test]
    fn bounded_byte_types_enforce_api_limits() {
        assert!(AssetName::try_from(Vec::new()).is_ok());
        assert!(AssetName::try_from(vec![0; 16]).is_ok());
        assert!(AssetName::try_from(vec![0; 32]).is_ok());
        assert!(AssetName::try_from(vec![0; 33]).is_err());
        assert!(MetadataBytes::try_from(Vec::new()).is_ok());
        assert!(MetadataBytes::try_from(vec![0; 32]).is_ok());
        assert!(MetadataBytes::try_from(vec![0; 64]).is_ok());
        assert!(MetadataBytes::try_from(vec![0; 65]).is_err());
        assert!(serde_json::from_str::<AssetName>("\"\"").is_ok());
        assert!(serde_json::from_str::<AssetName>("\"ff\"").is_ok());
        assert!(
            serde_json::from_str::<MetadataBytes>(&format!("\"{}\"", "00".repeat(65))).is_err()
        );
    }

    #[test]
    fn asset_ids_validate_map_keys() {
        let policy_id = Blake2b224([0xab; 28]);
        let asset_name = AssetName::try_from(vec![0xde, 0xad]).unwrap();
        let asset_id = AssetId::asset(policy_id, asset_name);
        let mut assets = BTreeMap::new();
        assets.insert(asset_id.clone(), 42);
        let value = Value { coins: 7, assets };

        let json = serde_json::to_string(&value).unwrap();
        let encoded_entry = format!("\"{}.dead\":42", "ab".repeat(28));
        assert!(json.contains(&encoded_entry));
        assert_eq!(
            serde_json::from_str::<Value>(&json)
                .unwrap()
                .assets
                .get(&asset_id),
            Some(&42)
        );

        let invalid = format!(
            r#"{{"coins":7,"assets":{{"{}.dead":42}}}}"#,
            "ab".repeat(27)
        );
        assert!(serde_json::from_str::<Value>(&invalid).is_err());
    }

    #[test]
    fn patterns_decode_hex_components_into_byte_types() {
        let policy = "ab".repeat(28);
        let transaction = "cd".repeat(32);

        assert_eq!(
            format!("{policy}.deadbeef").parse::<Pattern>().unwrap(),
            Pattern::asset(
                Blake2b224([0xab; 28]),
                AssetName::try_from(vec![0xde, 0xad, 0xbe, 0xef]).unwrap()
            )
        );
        assert_eq!(
            format!("7@{transaction}").parse::<Pattern>().unwrap(),
            Pattern::output_ref(7, Blake2b256([0xcd; 32]))
        );
        assert_eq!("*".parse::<Pattern>().unwrap(), Pattern::Wildcard);
        assert_eq!(
            "addr_test1example".parse::<Pattern>().unwrap(),
            Pattern::Address("addr_test1example".to_owned())
        );
        assert!(format!("7@{}", "cd".repeat(31)).parse::<Pattern>().is_err());
        assert!(
            format!("{}.xyz", "ab".repeat(28))
                .parse::<Pattern>()
                .is_err()
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Strict(pub bool);
