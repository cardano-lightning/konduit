//! Decides what to do each round of `tx::run`'s loop.

use konduit_data::Stage;
use rand::{Rng, RngExt};

use crate::config::AccountConfig;
use crate::receipt::Accounts as ReceiptSessions;
use crate::tx::{Action, Staging};

const CHANNEL_AMOUNT: u64 = 10_000_000;

const ADD_PROB: f64 = 0.3;
const CLOSE_PROB: f64 = 0.05;
const EVOLVE_CLAIM_PROB: f64 = 0.3;
const CLAIM_PROB: f64 = 0.15;

pub trait Strategy {
    fn choose(&mut self, staging: &mut Staging) -> anyhow::Result<()>;
}

#[derive(Clone, Copy)]
enum Turn {
    Consumer,
    Adaptor,
}

enum Phase {
    Up,
    Down,
}

/// Runs Up for the first `up_steps` rounds, then forces Down for good.
///
/// Up: opens any channel-less account; else picks a random turn. Consumer
/// adds sometimes, closes rarely. Adaptor evolves-then-claims on
/// Opened/Closed, else just claims.
///
/// Down: terminates channels by taking whatever step gets them there.
/// Per account, first success wins: End, Elapse, Expire, Close, then
/// (no evolving) Claim to unstick the adaptor side.
pub struct StepStrategy {
    up_steps: u32,
    round: u32,
    receipts: ReceiptSessions,
    receipt_ids: Vec<usize>,
}

impl StepStrategy {
    pub fn new(accounts: &[AccountConfig], up_steps: u32) -> Self {
        let mut receipts = ReceiptSessions::new(1.0);
        let receipt_ids = accounts
            .iter()
            .map(|a| receipts.insert(a.key.clone(), a.tag.clone(), CHANNEL_AMOUNT))
            .collect();
        Self {
            up_steps,
            round: 0,
            receipts,
            receipt_ids,
        }
    }

    fn claim(
        &mut self,
        staging: &mut Staging,
        account: &AccountConfig,
        id: usize,
    ) -> anyhow::Result<bool> {
        let receipt = self.receipts.yield_receipt(id);
        staging.try_apply_action(account, Action::Claim { receipt })
    }
}

impl Strategy for StepStrategy {
    fn choose(&mut self, staging: &mut Staging) -> anyhow::Result<()> {
        let phase = if self.round < self.up_steps {
            Phase::Up
        } else {
            Phase::Down
        };
        self.round += 1;

        let mut rng = rand::rng();
        let accounts = staging.accounts().to_vec();

        for (i, account) in accounts.iter().enumerate() {
            let vkey = account.verifying_key();
            let has_channel = staging.has_channel(&vkey);

            match phase {
                Phase::Up => {
                    if !has_channel {
                        staging.apply_action(
                            account,
                            Action::Open {
                                amount: CHANNEL_AMOUNT,
                            },
                        )?;
                        continue;
                    }

                    let turn = if rng.random_bool(0.5) {
                        Turn::Consumer
                    } else {
                        Turn::Adaptor
                    };
                    match turn {
                        Turn::Consumer => {
                            if rng.random_bool(ADD_PROB) {
                                staging.try_apply_action(
                                    account,
                                    Action::Add {
                                        amount: CHANNEL_AMOUNT,
                                    },
                                )?;
                            } else if rng.random_bool(CLOSE_PROB) {
                                staging.try_apply_action(account, Action::Close)?;
                            }
                        }
                        Turn::Adaptor => {
                            let opened_or_closed = matches!(
                                staging.stage(&vkey),
                                Some(Stage::Opened(..)) | Some(Stage::Closed(..))
                            );

                            if opened_or_closed {
                                if rng.random_bool(EVOLVE_CLAIM_PROB) {
                                    self.receipts.evolve_rng(self.receipt_ids[i], &mut rng, 1);
                                    self.claim(staging, account, self.receipt_ids[i])?;
                                }
                            } else if rng.random_bool(CLAIM_PROB) {
                                self.claim(staging, account, self.receipt_ids[i])?;
                            }
                        }
                    }
                }
                Phase::Down => {
                    if !has_channel {
                        continue;
                    }
                    if staging.try_apply_action(account, Action::End)? {
                        continue;
                    }
                    if staging.try_apply_action(account, Action::Elapse)? {
                        continue;
                    }
                    if staging.try_apply_action(account, Action::Expire)? {
                        continue;
                    }
                    if staging.try_apply_action(account, Action::Close)? {
                        continue;
                    }
                    self.claim(staging, account, self.receipt_ids[i])?;
                }
            }
        }

        Ok(())
    }
}
