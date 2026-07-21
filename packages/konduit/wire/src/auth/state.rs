use konduit_data::{Cheque, Squash, Stage};
use minicbor::{Decode, Encode};
use problem_details::ProblemDetail;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub const ENDPOINT: &str = "/state";
pub const PATH: &str = const_format::concatcp!(super::PATH, ENDPOINT);

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Params {
    /// A utxo a user specifies to indicate which thread to follow.
    /// A server may use this to select a lineage or thread,
    /// although they are not obliged to do so.
    /// In normal contexts (ie no mimics), this can be ommitted without consequence
    ///
    /// NOTE: this does offend  HTTP verb purists.
    /// Subsequent GET are impacted by this.
    #[n(0)]
    #[cfg_attr(
        feature = "serde",
        serde(flatten, skip_serializing_if = "Option::is_none")
    )]
    pub focus: Option<Input>,
}

#[derive(Debug, Clone, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Response {
    /// L1 data
    #[n(0)]
    pub backing: Backing,
    /// L2 data
    #[n(1)]
    pub receipt: Receipt,
}

pub type Error = super::Error<DomainError>;

#[derive(ProblemDetail)]
pub enum DomainError {
    /// FIXME :: Something went wrong.
    #[problem(slug = "state-other", title = "State Other", http_status = 400)]
    Other,
}

/// The backing consists of the thread of Utxos representing the L1 state of the channel
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Backing {
    /// If current is set to `[Option::None]`, then there are no recognized channel Utxos.
    #[n(0)]
    pub current: Option<ChannelUtxo>,
    #[n(1)]
    /// History is purely informational.
    /// The client can use this to support a richer UX with less state.
    /// The client may independently verify the data, which is easier reconstructing it from scratch.
    ///
    /// The server may truncate or prune history at anytime.
    /// The history does not indicate what is deemed settled or pending,
    /// however this may also dependent on the amount being committed.
    pub past: Vec<ChannelUtxo>,
    /// A utxo has been seen on-chain, but is not deemed settled.
    /// Funds here cannot be used to back pay commitments.
    #[n(2)]
    pub pending: Vec<ChannelUtxo>,
}

/// Date
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ChannelUtxo {
    #[n(0)]
    pub input: Input,
    #[n(1)]
    pub created_at: Point,
    /// In the case of Ada this is ALWAYS with MIN_ADA_BUFFER deducted
    /// from the actual amount of lovelace in the utxo value.
    #[n(2)]
    pub amount: u64,
    #[n(3)]
    pub stage: Stage,
}

/// A time indicator.
/// Posix time may refer to the block time slot.
/// It allows for a proxy on block depth without knowledge of current chain state.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Point {
    #[n(0)]
    pub posix: u64,
    /// Block height
    #[n(1)]
    pub block: u64,
}

/// As the user must register before accessing this endpoint,
/// a squash must be in posession of the server.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Receipt {
    #[n(0)]
    pub squash: Squash,
    #[n(1)]
    pub cheques: Vec<Cheque>,
}

/// Input (aka OutputReference) to identify a UTXO
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Input {
    #[n(0)]
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_with::As::<serde_with::hex::Hex>")
    )]
    pub output_reference: [u8; 32],
    #[n(1)]
    pub index: u64,
}
