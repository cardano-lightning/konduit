use std::cmp::min;

use konduit_data::{
    Cheque, Duration, Keytag, MixedReceipt, MixedReceiptUpdateError, Squash, Stage,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::models::L1Channel;

#[derive(Debug, PartialEq, Error)]
pub enum ChequeError {
    #[error("Channel not served")]
    NotServed,

    #[error("Bad signature")]
    BadSignature,

    #[error("Expires too soon")]
    ExpiresTooSoon,

    #[error("No L1 channel")]
    NoL1Channel,

    #[error("Channel not initiated")]
    NotInitiated,

    #[error("Channel stage not Opened")]
    NotOpened,

    #[error("Amount unavailable")]
    AmountUnavailable,
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum L2ChannelUpdateSquashError {
    #[error("Channel not served")]
    NotServed,

    #[error("Bad signature")]
    BadSignature,

    #[error("No L1 channel")]
    NoL1Channel,

    #[error("Channel stage not Opened")]
    NotOpened,

    #[error("Mixed Receipt Error {0}")]
    MixedReceipt(MixedReceiptUpdateError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2Channel {
    pub keytag: Keytag,
    /// L1Channel with greatest available funds.
    pub l1_channel: Option<L1Channel>,
    /// Current evidence of funds owed.
    pub mixed_receipt: Option<MixedReceipt>,
    /// L2 channel can be de-activated.
    pub is_served: bool,
}

impl L2Channel {
    pub fn new(keytag: Keytag, l1_channel: L1Channel) -> Self {
        L2Channel {
            keytag,
            l1_channel: Some(l1_channel),
            mixed_receipt: None,
            is_served: true,
        }
    }

    pub fn from_channels(keytag: Keytag, l1_channels: Vec<L1Channel>) -> Self {
        let l1_channel = l1_channels.into_iter().max_by_key(|item| match item.stage {
            konduit_data::Stage::Opened(_) => item.amount,
            _ => 0,
        });
        L2Channel {
            keytag,
            l1_channel,
            mixed_receipt: None,
            is_served: true,
        }
    }
}

impl L2Channel {
    /// Squash + Unlockeds
    pub fn owed(&self) -> u64 {
        self.mixed_receipt.as_ref().map_or(0, |x| x.amount())
    }

    /// Squash + All mixed cheques
    pub fn committed(&self) -> u64 {
        self.mixed_receipt.as_ref().map_or(0, |x| x.committed())
    }

    /// The amount the quote is for is available.
    pub fn can_quote(&self, amount: u64) -> bool {
        self.capacity() > 0 && self.available() >= amount
    }

    /// How many more cheques can be issued (while none are squashed)
    /// before the channel has no capacity.
    pub fn capacity(&self) -> usize {
        self.mixed_receipt.as_ref().map_or(0, |x| x.capacity())
    }

    pub fn available(&self) -> u64 {
        if !self.is_served || self.mixed_receipt.is_none() {
            return 0;
        }
        let Some(l1_channel) = &self.l1_channel else {
            return 0;
        };
        let konduit_data::Stage::Opened(subbed) = l1_channel.stage else {
            return 0;
        };
        let committed = self.committed();
        if committed < subbed {
            // This should happen only if there exists mimics
            return 0;
        }
        let rel_committed = committed - subbed;
        let held = l1_channel.amount;
        if rel_committed > held {
            // This should happen only if there exists mimics
            return 0;
        }
        held - rel_committed
    }

    /// Find the L1 with max claimable amount, max avaliable amount
    pub fn update_from_l1(&mut self, channels: Vec<L1Channel>) {
        let owed = self.owed();
        let l1_channel = channels.iter().max_by_key(|item| match item.stage {
            Stage::Opened(subbed) => {
                if owed < subbed {
                    (0, 0)
                } else {
                    (min(owed - subbed, item.amount), item.amount)
                }
            }
            _ => (0, 0),
        });
        self.l1_channel = l1_channel.cloned();
    }

    pub fn add_cheque(&mut self, cheque: Cheque, timeout: Duration) -> Result<(), ChequeError> {
        if !self.is_served {
            return Err(ChequeError::NotServed);
        };
        let (key, tag) = self.keytag.split();
        if cheque.verify(&key, &tag) {
            return Err(ChequeError::BadSignature);
        }
        if cheque.cheque_body.timeout >= timeout {
            return Err(ChequeError::ExpiresTooSoon);
        }
        let Some(l1_channel) = self.l1_channel.as_ref() else {
            return Err(ChequeError::NoL1Channel);
        };
        let Some(ref mut mixed_receipt) = self.mixed_receipt else {
            return Err(ChequeError::NotInitiated);
        };
        let subbed = if let konduit_data::Stage::Opened(subbed_val) = l1_channel.stage {
            subbed_val
        } else {
            return Err(ChequeError::NotOpened);
        };
        let committed = mixed_receipt.committed();
        let available = if committed > subbed {
            std::cmp::max(committed - subbed, l1_channel.amount)
        } else {
            0
        };
        if available > cheque.cheque_body.amount {
            return Err(ChequeError::AmountUnavailable);
        }
        // FIXME :: HANDLE ERROR
        mixed_receipt.insert(cheque).unwrap();
        Ok(())
    }

    pub fn update_squash(&mut self, squash: Squash) -> Result<bool, L2ChannelUpdateSquashError> {
        if !self.is_served {
            return Err(L2ChannelUpdateSquashError::NotServed);
        };
        let (key, tag) = self.keytag.split();
        if !squash.verify(&key, &tag) {
            return Err(L2ChannelUpdateSquashError::BadSignature);
        }
        let Some(l1_channel) = self.l1_channel.as_ref() else {
            return Err(L2ChannelUpdateSquashError::NoL1Channel);
        };
        let Stage::Opened(_) = l1_channel.stage else {
            return Err(L2ChannelUpdateSquashError::NotOpened);
        };
        let Some(ref mut mixed_receipt) = self.mixed_receipt.as_mut() else {
            self.mixed_receipt = Some(MixedReceipt::new(squash, vec![]).unwrap());
            return Ok(true);
        };
        mixed_receipt
            .update(squash)
            .map_err(L2ChannelUpdateSquashError::MixedReceipt)
    }
}
