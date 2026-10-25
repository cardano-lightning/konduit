//! JSON-output DTOs for the CLI commands.
//!
//! These types are intentionally defined locally in the CLI package so that the
//! underlying crates (`konduit-data`, `konduit-tmp`, `cardano-sdk`) are not
//! forced to commit to a specific wire shape. Every type in this module
//! derives both [`serde::Serialize`] and [`serde::Deserialize`] so values can be
//! round-tripped by downstream callers (e.g. the TS integration package).

use std::collections::BTreeMap;

use cardano_sdk::{Address, Hash, Input, Output, Value, address};
use konduit_data::{Constants, Pending, Stage, Tag, Used};
use konduit_tmp::Keytag;
use konduit_tx::Channel as KonduitChannel;
use serde::{Deserialize, Serialize};

/// Console-style flags that a command supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputJson(pub bool);

impl OutputJson {
    pub fn is_json(self) -> bool {
        self.0
    }
}

/// Render `value` either as pretty JSON or as text depending on `output`.
pub fn render<T>(output: OutputJson, value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    if output.is_json() {
        let json = serde_json::to_string_pretty(value)?;
        println!("{}", json);
    }
    Ok(())
}

/// A wallet or destination address as a bech32 string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AddressDto(#[serde(with = "bech32_via_display")] String);

impl AddressDto {
    pub fn new<T: address::IsAddressKind>(address: &Address<T>) -> Self {
        Self(address.to_string())
    }
}

/// Output of `admin show address` / `consumer show address` / `adaptor show address`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddressOutput {
    pub address: AddressDto,
}

impl AddressOutput {
    pub fn from_admin(config: &crate::config::admin::Config) -> anyhow::Result<Self> {
        let network = config.connector.network().into();
        let address = config.wallet.to_verification_key().to_address(network);
        Ok(Self {
            address: AddressDto::new(&address),
        })
    }

    pub fn from_consumer(config: &crate::config::consumer::Config) -> anyhow::Result<Self> {
        let network = config.connector.network().into();
        let address = config.wallet.to_verification_key().to_address(network);
        Ok(Self {
            address: AddressDto::new(&address),
        })
    }

    pub fn from_adaptor(config: &crate::config::adaptor::Config) -> anyhow::Result<Self> {
        let network = config.connector.network().into();
        let address = config.wallet.to_verification_key().to_address(network);
        Ok(Self {
            address: AddressDto::new(&address),
        })
    }
}

/// Constants used to identify a channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ConstantsOutput {
    pub tag: String,
    pub add_vkey: String,
    pub sub_vkey: String,
    pub close_period_ms: u64,
}

#[allow(dead_code)]
impl From<&Constants> for ConstantsOutput {
    fn from(c: &Constants) -> Self {
        Self {
            tag: c.tag.to_string(),
            add_vkey: hex::encode(c.add_vkey.to_bytes()),
            sub_vkey: hex::encode(c.sub_vkey.to_bytes()),
            close_period_ms: u64::from(c.close_period),
        }
    }
}

/// Output of `adaptor show constants`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptorConstantsOutput {
    pub sub_vkey: String,
    pub close_period_ms: u64,
}

impl AdaptorConstantsOutput {
    pub fn from_config(config: &crate::config::adaptor::Config) -> Self {
        Self {
            sub_vkey: hex::encode(config.wallet.to_verification_key().into_bytes()),
            close_period_ms: u64::from(config.close_period),
        }
    }
}

/// Output of `consumer show keytag`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeytagOutput {
    /// Hex-encoded keytag (32-byte verifier prefix + tag bytes).
    pub keytag: String,
    /// Hex-encoded verifier key encoded in the keytag.
    pub verification_key: String,
    /// Hex-encoded tag bytes.
    pub tag: String,
}

impl KeytagOutput {
    pub fn new(key: &cardano_sdk::VerificationKey, tag: &Tag) -> Self {
        let keytag = Keytag::new(key, tag);
        Self {
            keytag: keytag.to_string(),
            verification_key: hex::encode(<[u8; 32]>::from(*key)),
            tag: tag.to_string(),
        }
    }
}

/// Output of `consumer make squash`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SquashOutput {
    /// Hex of the cbor-encoded squash body.
    pub body_cbor_hex: String,
    /// Hex of the signature over `tag + body`.
    pub signature_hex: String,
    /// Total amount squashed (top-level convenience field).
    pub amount: u64,
    /// Highest index included in the squash.
    pub index: u64,
}

impl SquashOutput {
    pub fn from_squash(squash: &konduit_data::Squash) -> anyhow::Result<Self> {
        let body_bytes = minicbor::to_vec(squash.body()).unwrap_or_default();
        Ok(Self {
            body_cbor_hex: hex::encode(body_bytes),
            signature_hex: hex::encode(squash.signature().to_bytes()),
            amount: squash.amount(),
            index: squash.index(),
        })
    }
}

/// Output of `consumer make locked`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedOutput {
    /// Hex of the cbor-encoded cheque body.
    pub body_cbor_hex: String,
    /// Hex of the signature over `tag + body`.
    pub signature_hex: String,
    /// Hex of the lock (sha256 of the secret).
    pub lock_hex: String,
    /// Cheque index.
    pub index: u64,
    /// Cheque amount (lovelace).
    pub amount: u64,
    /// Absolute timeout (POSIX ms).
    pub timeout_ms: u64,
}

impl LockedOutput {
    pub fn from_locked(locked: &konduit_data::Locked) -> anyhow::Result<Self> {
        let body_bytes = minicbor::to_vec(locked.body()).unwrap_or_default();
        let body = locked.body();
        Ok(Self {
            body_cbor_hex: hex::encode(body_bytes),
            signature_hex: hex::encode(locked.signature().to_bytes()),
            lock_hex: hex::encode(body.lock().0),
            index: body.index(),
            amount: body.amount(),
            timeout_ms: u64::from(body.timeout()),
        })
    }
}

/// Output of any transaction-building command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxOutput {
    /// Hex-encoded transaction id.
    pub tx_id: String,
}

impl TxOutput {
    pub fn from_id(tx_id: String) -> Self {
        Self { tx_id }
    }
}

/// Per-channel projection used in tip outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelOutput {
    pub tag: String,
    pub add_vkey: String,
    pub sub_vkey: String,
    pub close_period_ms: u64,
    pub amount_lovelace: u64,
    pub buffered_lovelace: u64,
    pub keytag_hex: String,
    pub stage: ChannelStage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "label", rename_all = "snake_case")]
pub enum ChannelStage {
    Opened {
        subbed: u64,
        useds: Vec<UsedDto>,
    },
    Closed {
        subbed: u64,
        useds: Vec<UsedDto>,
        elapse_at_ms: u64,
    },
    Responded {
        pendings_amount: u64,
        pendings: Vec<PendingDto>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsedDto {
    pub index: u64,
    pub amount: u64,
}

impl From<&Used> for UsedDto {
    fn from(used: &Used) -> Self {
        Self {
            index: used.index,
            amount: used.amount,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingDto {
    pub amount: u64,
    pub timeout_ms: u64,
    pub lock_hex: String,
}

impl From<&Pending> for PendingDto {
    fn from(pending: &Pending) -> Self {
        Self {
            amount: pending.amount,
            timeout_ms: u64::from(pending.timeout),
            lock_hex: hex::encode(pending.lock.0),
        }
    }
}

impl ChannelStage {
    pub fn from_konduit(stage: &Stage) -> Self {
        match stage {
            Stage::Opened(subbed, useds) => Self::Opened {
                subbed: *subbed,
                useds: useds.iter().map(UsedDto::from).collect(),
            },
            Stage::Closed(subbed, useds, duration) => Self::Closed {
                subbed: *subbed,
                useds: useds.iter().map(UsedDto::from).collect(),
                elapse_at_ms: u64::from(*duration),
            },
            Stage::Responded(pends_amount, pendings) => Self::Responded {
                pendings_amount: *pends_amount,
                pendings: pendings.iter().map(PendingDto::from).collect(),
            },
        }
    }
}

impl ChannelOutput {
    pub fn from_konduit(channel: &KonduitChannel) -> Self {
        let constants = channel.constants();
        Self {
            tag: constants.tag.to_string(),
            add_vkey: hex::encode(constants.add_vkey.to_bytes()),
            sub_vkey: hex::encode(constants.sub_vkey.to_bytes()),
            close_period_ms: u64::from(constants.close_period),
            amount_lovelace: channel.amount(),
            buffered_lovelace: channel.buffered_amount(),
            keytag_hex: channel.keytag().to_string(),
            stage: ChannelStage::from_konduit(channel.stage()),
        }
    }
}

/// Summary of wallet UTxOs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WalletSummary {
    pub utxo_count: usize,
    pub total_lovelace: u64,
    pub reference_script: Option<ReferenceScriptDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceScriptDto {
    pub tx_id_hex: String,
    pub output_index: u64,
    pub script_version: String,
    pub script_hash_hex: String,
}

impl ReferenceScriptDto {
    pub fn from_utxo(input: &Input, output: &Output) -> anyhow::Result<Self> {
        let script = output
            .script()
            .ok_or_else(|| anyhow::anyhow!("reference script missing on utxo"))?;
        Ok(Self {
            tx_id_hex: hex::encode(input.transaction_id()),
            output_index: input.output_index(),
            script_version: format!("{:?}", script.version()),
            script_hash_hex: hex::encode(Hash::<28>::from(script).as_ref()),
        })
    }
}

impl WalletSummary {
    pub fn from_utxos(
        utxos: &BTreeMap<Input, Output>,
        reference_script: &Option<(Input, Output)>,
    ) -> anyhow::Result<Self> {
        let utxo_count = utxos.len();
        let total_lovelace = utxos
            .values()
            .fold(Value::new(0), |acc, output| {
                let mut v = acc;
                v.add(output.value());
                v
            })
            .lovelace();
        let reference_script = reference_script
            .as_ref()
            .map(|(input, output)| ReferenceScriptDto::from_utxo(input, output))
            .transpose()?;
        Ok(Self {
            utxo_count,
            total_lovelace,
            reference_script,
        })
    }
}

/// Admin tip - only wallet & reference script (no channels).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminTipOutput {
    pub wallet: WalletSummary,
    pub reference_script: Option<ReferenceScriptDto>,
}

/// Consumer tip - wallet, reference script and owned channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerTipOutput {
    pub wallet: WalletSummary,
    pub reference_script: Option<ReferenceScriptDto>,
    pub channels: Vec<ChannelOutput>,
}

impl ConsumerTipOutput {
    pub fn from_parts(
        wallet: &BTreeMap<Input, Output>,
        reference_script: &Option<(Input, Output)>,
        channels: &BTreeMap<Input, KonduitChannel>,
    ) -> anyhow::Result<Self> {
        let wallet = WalletSummary::from_utxos(wallet, reference_script)?;
        let reference_script = reference_script
            .as_ref()
            .map(|(i, o)| ReferenceScriptDto::from_utxo(i, o))
            .transpose()?;
        let channels = channels.values().map(ChannelOutput::from_konduit).collect();
        Ok(Self {
            wallet,
            reference_script,
            channels,
        })
    }
}

/// Adaptor tip - wallet, reference script and watched channels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdaptorTipOutput {
    pub wallet: WalletSummary,
    pub reference_script: Option<ReferenceScriptDto>,
    pub channels: Vec<ChannelOutput>,
}

impl AdaptorTipOutput {
    pub fn from_parts(
        wallet: &BTreeMap<Input, Output>,
        reference_script: &Option<(Input, Output)>,
        channels: &BTreeMap<Input, KonduitChannel>,
    ) -> anyhow::Result<Self> {
        let wallet = WalletSummary::from_utxos(wallet, reference_script)?;
        let reference_script = reference_script
            .as_ref()
            .map(|(i, o)| ReferenceScriptDto::from_utxo(i, o))
            .transpose()?;
        let channels = channels.values().map(ChannelOutput::from_konduit).collect();
        Ok(Self {
            wallet,
            reference_script,
            channels,
        })
    }
}

// Bech32 v.s. raw address. We accept and emit plain bech32 strings.
mod bech32_via_display {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &str, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn used_dto_roundtrip() {
        let used = Used {
            index: 7,
            amount: 999,
        };
        let dto = UsedDto::from(&used);
        let json = serde_json::to_string(&dto).unwrap();
        let back: UsedDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
        assert_eq!(dto.index, used.index);
        assert_eq!(dto.amount, used.amount);
    }

    #[test]
    fn stage_serialization_uses_label_tag() {
        let stage = Stage::Opened(42, vec![Used::new(1, 100)]);
        let dto = ChannelStage::from_konduit(&stage);
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("\"label\":\"opened\""), "got {json}");
        let back: ChannelStage = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }

    #[test]
    fn squash_body_dto_roundtrip() {
        let body = konduit_data::SquashBody::new_no_verify(
            1234,
            5,
            konduit_data::Indexes::new(vec![1, 2, 3]).unwrap(),
        );
        let dto = SquashOutput::from_squash(&konduit_data::Squash::new(
            body,
            konduit_data::Signature::from([1; 64]),
        ))
        .unwrap();
        let json = serde_json::to_string(&dto).unwrap();
        let back: SquashOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }

    #[test]
    fn pending_dto_serializes_lock_as_hex() {
        let pending = Pending::new(
            7,
            konduit_data::Duration::from_secs(0),
            konduit_data::Lock([1; 32]),
        );
        let dto = PendingDto::from(&pending);
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains(&"01".repeat(32)), "got {json}");
        let back: PendingDto = serde_json::from_str(&json).unwrap();
        assert_eq!(dto, back);
    }
}
