//! Type definitions shared between Kupo endpoints.
//!
//! These mirror the JSON shapes described in the [Kupo API reference](https://cardanosolutions.github.io/kupo/).

use std::collections::BTreeMap;
use std::fmt;

/// A blake2b-224 hash digest, base16-encoded. Used for script hashes.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
#[serde(transparent)]
pub struct Hash28(pub String);

impl fmt::Display for Hash28 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A blake2b-256 hash digest, base16-encoded. Used for transaction, datum, header hashes.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize, serde::Serialize,
)]
#[serde(transparent)]
pub struct Hash32(pub String);

impl fmt::Display for Hash32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A reference to a block on the chain: slot number + header hash.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Point {
    pub slot_no: u64,
    pub header_hash: Hash32,
}

/// A (multi-asset) value of a transaction output.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub struct Value {
    pub coins: u64,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub assets: BTreeMap<String, u64>,
}

/// A resolved Plutus' datum.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Datum {
    pub datum: String,
}

/// The type of datum referenced in an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatumType {
    Hash,
    Inline,
}

/// A resolved native or Plutus script.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Script {
    pub language: ScriptLanguage,
    pub script: String,
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct SpentAt {
    pub slot_no: u64,
    pub header_hash: Hash32,
    pub transaction_id: Option<Hash32>,
    pub input_index: Option<u64>,
    pub redeemer: Option<String>,
}

/// A single match: a transaction output matched by a pattern.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Match {
    pub transaction_index: u64,
    pub transaction_id: Hash32,
    pub output_index: u64,
    pub address: String,
    pub value: Value,
    pub datum_hash: Option<Hash32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datum_type: Option<DatumType>,
    pub script_hash: Option<Hash28>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub script: Option<Script>,
    pub created_at: Point,
    pub spent_at: Option<SpentAt>,
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
    /// An asset id pattern: a policy id (hex) and an optional asset name (hex).
    AssetId {
        policy_id: String,
        asset_name: AssetNamePattern,
    },
    /// An output reference pattern: an optional output index and a transaction id (hex).
    OutputReference {
        index: Option<u64>,
        transaction_id: String,
    },
}

/// The asset name part of an [`Pattern::AssetId`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetNamePattern {
    /// Match any asset name under the given policy.
    Wildcard,
    /// Match a specific asset name, base16-encoded.
    Name(String),
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
    pub fn policy(policy_id: impl Into<String>) -> Self {
        Self::AssetId {
            policy_id: policy_id.into(),
            asset_name: AssetNamePattern::Wildcard,
        }
    }

    /// An asset id pattern matching a specific policy and asset name.
    pub fn asset(policy_id: impl Into<String>, asset_name: impl Into<String>) -> Self {
        Self::AssetId {
            policy_id: policy_id.into(),
            asset_name: AssetNamePattern::Name(asset_name.into()),
        }
    }

    /// An output reference pattern matching a specific transaction id, any output index.
    pub fn tx(transaction_id: impl Into<String>) -> Self {
        Self::OutputReference {
            index: None,
            transaction_id: transaction_id.into(),
        }
    }

    /// An output reference pattern matching a specific transaction id and output index.
    pub fn output_ref(index: u64, transaction_id: impl Into<String>) -> Self {
        Self::OutputReference {
            index: Some(index),
            transaction_id: transaction_id.into(),
        }
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
                f.write_str(policy_id)?;
                f.write_str(".")?;
                match asset_name {
                    AssetNamePattern::Wildcard => f.write_str("*"),
                    AssetNamePattern::Name(name) => f.write_str(name),
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
                f.write_str(transaction_id)
            }
        }
    }
}

impl<'de> serde::Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Ok(Pattern::Address(raw))
    }
}

impl serde::Serialize for Pattern {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
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
    Point { slot_no: u64, header_hash: Hash32 },
}

impl fmt::Display for MatchBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Slot(slot) => write!(f, "{}", slot),
            Self::Point {
                slot_no,
                header_hash,
            } => write!(f, "{}.{}", slot_no, header_hash.0),
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
    pub policy_id: Option<Hash28>,
    pub asset_name: Option<String>,
    pub transaction_id: Option<Hash32>,
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
    pub fn with_policy(mut self, policy_id: Hash28) -> Self {
        self.policy_id = Some(policy_id);
        self
    }

    /// Restrict matches to outputs containing the given asset name. Must be combined with a `policy_id`.
    pub fn with_asset_name(mut self, name: impl Into<String>) -> Self {
        self.asset_name = Some(name.into());
        self
    }

    /// Restrict matches to outputs at the given transaction id.
    pub fn with_transaction(mut self, tx_id: Hash32) -> Self {
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
    pub header_hash: Option<Hash32>,
}

impl RollbackTo {
    pub fn slot(slot_no: u64) -> Self {
        Self {
            slot_no,
            header_hash: None,
        }
    }

    pub fn point(slot_no: u64, header_hash: Hash32) -> Self {
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
    pub header_hash: Option<&'a str>,
}

impl<'a> From<&'a ForcedRollback> for ForcedRollbackBody<'a> {
    fn from(rollback: &'a ForcedRollback) -> Self {
        Self {
            rollback_to: RollbackToBody {
                slot_no: rollback.rollback_to.slot_no,
                header_hash: rollback
                    .rollback_to
                    .header_hash
                    .as_ref()
                    .map(|h| h.0.as_str()),
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
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
    pub hash: Hash32,
    pub schema: Metadatum,
    pub raw: String,
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
    pub bytes: String,
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
