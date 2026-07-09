//! # Building a transaction
//!
//! This crate does not provide a single entry point for stepping channels —
//! callers assemble the pipeline themselves, since the shape of "user intent"
//! differs by role (e.g. an adaptor acting on receipts vs. a consumer acting
//! on its own requests).
//!
//! 1. **Source on-chain state.** Parse chain UTxOs into [`ChannelUtxo`]s via
//!    `ChannelUtxo::try_from`, filtering to the channels relevant to the
//!    caller (e.g. by `sub_vkey` or `add_vkey`).
//!
//! 2. **Source intent.** Gather whatever represents the caller's proof or
//!    request for each channel — e.g. a receipt (squash/cheques/secrets) for
//!    an adaptor, or an [`Intent`] for a consumer. This crate has no opinion
//!    on the shape; it only consumes the parts each [`Channel`] method needs.
//!
//! 3. **Step and collect.** For each `ChannelUtxo`, inspect `stage()` to pick
//!    the applicable method (`sub`/`respond`/`unlock` for an adaptor acting
//!    on receipts; `add`/`close`/`elapse`/`expire`/`end` for a consumer
//!    acting on its own intents), filter out failures or insufficient gain,
//!    and collect into [`SteppedUtxos`].
//!
//! 4. **Build the transaction.** Pass the `SteppedUtxos`, along with any
//!    [`Open`]s, fuel UTxOs, and a reference script, to [`tx::tx`].
//!
//! See `adaptor.rs` and `consumer.rs` (elsewhere in the workspace) for the
//! two concrete instantiations of this pipeline.

/// Konduit validator
mod validator;
pub use validator::*;

/// Generic containers
mod utxo;
pub use utxo::Utxo;

mod utxo_and;
pub use utxo_and::UtxoAnd;

mod utxos;
pub use utxos::Utxos;

/// Time
mod bounds;
pub use bounds::Bounds;

/// State
mod variables;
pub use variables::Variables;

mod channel;
pub use channel::Channel;

// Intents/ actions
mod open;
pub use open::Open;

mod step_to;
pub use step_to::StepTo;

// Actions and Outcome
mod stepped;
pub use stepped::Stepped;

mod step_error;
pub use step_error::StepError;

// Paired with utxos
mod channel_utxo;
pub use channel_utxo::ChannelUtxo;

mod stepped_utxo;
pub use stepped_utxo::SteppedUtxo;

mod stepped_utxos;
pub use stepped_utxos::SteppedUtxos;

/// Network params
mod network_parameters;
pub use network_parameters::NetworkParameters;

/// Fuel / wallet utxos
pub mod fuel;

/// Tx
pub mod tx;

// pub mod adaptor;
// pub use adaptor::InsufficientTotalGain;
// pub mod admin;
// pub mod consumer;
