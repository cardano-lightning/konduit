mod adaptor_info;
mod channel_parameters;
mod l1_channel;
mod pay_body;
mod possible_step;
mod quote;
mod quote_body;
pub mod receipt;
mod squash_proposal;
mod squash_status;

pub use adaptor_info::*;
pub use channel_parameters::ChannelParameters;
pub use l1_channel::L1Channel;
pub use pay_body::PayBody;
pub use possible_step::PossibleStep;
pub use quote::Quote;
pub use quote_body::QuoteBody;
pub use receipt::Receipt;
pub use squash_proposal::SquashProposal;
pub use squash_status::SquashStatus;

mod keytag;
mod posix_seconds;

pub use keytag::Keytag;
pub use posix_seconds::PosixSeconds;

// Conversions.
// FIXME :: put these elsewhere
pub fn to_verifying_key(vk: cardano_sdk::VerificationKey) -> konduit_data::VerifyingKey {
    <[u8; 32]>::from(vk).into()
}

pub fn from_verifying_key(vk: konduit_data::VerifyingKey) -> cardano_sdk::VerificationKey {
    <[u8; 32]>::from(vk).into()
}
