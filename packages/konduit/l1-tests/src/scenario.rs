use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub mod l2_resolver;

mod cheque;
use cheque::Stub;

pub mod schedule;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    /// TODO: Not sure how this should work
    pub on_overrun: OnOverrun,
    /// If no channels are seen, open all channels.
    #[serde(default)]
    pub opens: Vec<u64>,
    #[serde(default)]
    pub l1: Vec<Tx>,
    #[serde(default)]
    pub l2: Vec<Vec<Stub>>,
}

impl Default for Config {
    fn default() -> Self {
        let n = 3;
        let amount = 10_000_000;
        let opens = vec![amount.clone(); n];
        let cheques = vec![
            vec![
                Stub::locked(2, 1, 10),
                Stub::unlocked(3, 1, 5, 10),
                Stub::squashed(4, 3, 5, 4, 0),
            ],
            vec![Stub::unlocked(3, 1, 5, 10), Stub::squashed(4, 3, 5, 4, 0)],
            vec![
                Stub::squashed(1, 1, 5, 10, 2),
                Stub::squashed(4, 3, 5, 4, 0),
            ],
        ];
        let txs = vec![
            Tx::adaptor(n),
            Tx::adaptor(n),
            Tx::consumer(n),
            Tx::adaptor(n),
            Tx::consumer(n),
            Tx::consumer(n),
        ];
        Self {
            on_overrun: OnOverrun::Clamp,
            opens,
            l2: cheques,
            l1: txs,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnOverrun {
    #[default]
    Error,
    /// Clamps to the available budget instead of erroring; always
    /// `tracing::warn!`s when it actually clamps something.
    Clamp,
}

impl fmt::Display for OnOverrun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OnOverrun::Error => write!(f, "error"),
            OnOverrun::Clamp => write!(f, "clamp"),
        }
    }
}

/// Account index into `Config::accounts`. `toml` only deserializes table
/// keys as `String`, so this recovers `usize` via `try_from`/`into`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AccountIndex(pub usize);

impl TryFrom<String> for AccountIndex {
    type Error = std::num::ParseIntError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse().map(AccountIndex)
    }
}

impl From<AccountIndex> for String {
    fn from(idx: AccountIndex) -> String {
        idx.0.to_string()
    }
}

impl fmt::Display for AccountIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// One entry, in file order. A `Tx`'s keys are account indices; at most
/// one action per account per tx (structural, via `BTreeMap`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tx {
    Consumer(BTreeMap<AccountIndex, ConsumerStep>),
    Adaptor(BTreeMap<AccountIndex, AdaptorStep>),
    Skip,
}

impl Tx {
    pub fn consumer(l: usize) -> Self {
        Self::Consumer(
            (0..l)
                .map(|i| (AccountIndex(i), ConsumerStep::default()))
                .collect(),
        )
    }

    pub fn adaptor(l: usize) -> Self {
        Self::Adaptor(
            (0..l)
                .map(|i| (AccountIndex(i), AdaptorStep::default()))
                .collect(),
        )
    }
}

fn fmt_steps<T: fmt::Display>(
    f: &mut fmt::Formatter<'_>,
    kind: &str,
    steps: &BTreeMap<AccountIndex, T>,
) -> fmt::Result {
    write!(f, "{kind} {{ ")?;
    for (i, (account, step)) in steps.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{account}: {step}")?;
    }
    write!(f, " }}")
}

impl fmt::Display for Tx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tx::Consumer(steps) => fmt_steps(f, "consumer", steps),
            Tx::Adaptor(steps) => fmt_steps(f, "adaptor", steps),
            Tx::Skip => write!(f, "skip"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerStep {
    // Advance to next stage or terminate if possible.
    #[default]
    Step,
    Add(u64),
    Close,
    Elapse,
    Expire,
    End,
}

impl ConsumerStep {
    pub fn step() -> Self {
        Self::Step
    }
}

impl fmt::Display for ConsumerStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsumerStep::Step => write!(f, "step"),
            ConsumerStep::Add(amount) => write!(f, "add({amount})"),
            ConsumerStep::Close => write!(f, "close"),
            ConsumerStep::Elapse => write!(f, "elapse"),
            ConsumerStep::Expire => write!(f, "expire"),
            ConsumerStep::End => write!(f, "end"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdaptorStep {
    #[default]
    Claim,
}

impl fmt::Display for AdaptorStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdaptorStep::Claim => write!(f, "claim"),
        }
    }
}
