use serde::{Deserialize, Serialize};
use tracing::{debug, instrument, warn};

use konduit_data::{ChequeBody, Duration, Lock, Locked, Secret, SquashBodyError, Tag, Verified};
use konduit_tmp::{Receipt, receipt};

use crate::{
    account, hash32, now,
    scenario::schedule::{Kind, Schedule},
};

static ADA: u64 = 1_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tick_ms: u64,
    pub amount_scale: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tick_ms: 60_000,
            amount_scale: ADA,
        }
    }
}

fn deterministic_secret(tag: &Tag, index: u64, amount: u64) -> Secret {
    let mut bytes = tag.as_ref().to_vec();
    bytes.extend(index.to_le_bytes());
    bytes.extend(amount.to_le_bytes());
    Secret(hash32(&bytes))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("incoherent")]
    Incoherent,
    #[error("receipt: {0}")]
    Receipt(#[from] receipt::Error),
    #[error("squash: {0}")]
    Squash(#[from] SquashBodyError),
}

pub struct L2Resolver {
    config: Config,
    account: account::Config,
    schedule: Schedule,
    /// Zero tick at — runtime, may be updated mid-run.
    zero: Duration,
    tick: usize,
    receipt: Receipt,
}

impl L2Resolver {
    /// Canonical constructor — everything explicit, nothing implied.
    /// All other constructors delegate here.
    fn from_parts(
        tick: usize,
        config: Config,
        account: account::Config,
        schedule: Schedule,
        zero: Duration,
        receipt: Receipt,
    ) -> Self {
        Self {
            tick,
            config,
            account,
            schedule,
            zero,
            receipt,
        }
    }

    /// Fresh start, zero = provided instant. Applies tick 0's events.
    pub fn new(
        config: Config,
        account: account::Config,
        schedule: Schedule,
        zero: Duration,
    ) -> Result<Self, Error> {
        let receipt = account.new_receipt();
        let mut resolver = Self::from_parts(0, config, account, schedule, zero, receipt);
        resolver.apply_current_tick()?;
        Ok(resolver)
    }

    /// Fresh start, zero = now.
    pub fn starting_now(
        config: Config,
        account: account::Config,
        schedule: Schedule,
    ) -> Result<Self, Error> {
        Self::new(config, account, schedule, now())
    }

    /// Resume from a snapshot — tick/receipt already reflect events up to `tick`.
    pub fn resume(
        tick: usize,
        config: Config,
        account: account::Config,
        schedule: Schedule,
        zero: Duration,
        receipt: Receipt,
    ) -> Self {
        Self::from_parts(tick, config, account, schedule, zero, receipt)
    }

    /// Call this whenever the runtime learns a new `zero` mid-run.
    pub fn set_zero(&mut self, zero: Duration) {
        self.zero = zero;
    }

    fn to_duration(&self, tick: usize) -> Duration {
        Duration::from_millis(self.zero.as_millis() as u64 + self.config.tick_ms * tick as u64)
    }

    fn locked_body(&self, tick: usize, index: u64, amount: u64) -> ChequeBody<Lock> {
        let timeout = self.to_duration(tick);
        let lock = Lock::from(deterministic_secret(self.account.tag(), index, amount));
        ChequeBody::new(index, amount, timeout, lock)
    }

    pub fn locked(&self, tick: usize, index: u64, amount: u64) -> Locked<Verified> {
        self.account
            .locked_inner(self.locked_body(tick, index, amount))
    }

    pub fn receipt(&self) -> &Receipt {
        &self.receipt
    }

    #[instrument(skip(self), fields(now = self.tick))]
    fn lock(&mut self, index: u64, amount: u64, timeout: usize) -> Result<(), Error> {
        debug!(index, amount, timeout, "applying lock");
        let locked = self.locked(timeout, index, amount);
        self.receipt.apply_locked(locked)?;
        Ok(())
    }

    #[instrument(skip(self), fields(now = self.tick))]
    fn unlock(&mut self, index: u64) -> Result<(), Error> {
        debug!(index, "applying unlock");
        let Some(l) = self.receipt.lockeds().find(|l| l.index() == index) else {
            warn!(
                index,
                now = self.tick,
                "unlock has no matching locked cheque — schedule/receipt out of sync"
            );
            return Err(Error::Incoherent);
        };
        let secret = deterministic_secret(self.account.tag(), index, l.amount());
        self.receipt.apply_secret(secret)?;
        Ok(())
    }

    /// Timeout is being applied to the whole receipt.
    /// Unclear whether this is desirable, or even important.
    #[instrument(skip(self), fields(now = self.tick))]
    fn timeout(&mut self, _index: u64) -> Result<(), Error> {
        debug!("applying timeout to whole receipt");
        self.receipt.apply_timeout(self.to_duration(self.tick));
        Ok(())
    }

    #[instrument(skip(self), fields(now = self.tick))]
    fn squash(&mut self, index: u64) -> Result<(), Error> {
        debug!(index, "applying squash");
        let Some(unlocked) = self.receipt.unlockeds().find(|u| u.index() == index) else {
            warn!(
                index,
                now = self.tick,
                "squash has no matching unlocked cheque — schedule/receipt out of sync"
            );
            return Err(Error::Incoherent);
        };
        let mut body = self.receipt.squash().body().clone();
        body.squash_unlocked(&unlocked)?;
        self.receipt.apply_squash(self.account.squash(body))?;
        Ok(())
    }

    #[instrument(skip(self), fields(now = self.tick))]
    fn apply_current_tick(&mut self) -> Result<(), Error> {
        let events = self.schedule.get(&self.tick).cloned().unwrap_or_default();
        debug!(count = events.len(), "events at tick");
        for event in events.iter() {
            match event.kind {
                Kind::Lock { amount, timeout } => {
                    self.lock(event.idx, amount * self.config.amount_scale, timeout)?
                }
                Kind::Unlock => self.unlock(event.idx)?,
                Kind::Squash => self.squash(event.idx)?,
            }
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<(), Error> {
        self.tick += 1;
        self.apply_current_tick()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scenario::cheque::Stub;
    use konduit_data::SigningKey;

    fn trace() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
    }

    fn test_config() -> Config {
        Config {
            tick_ms: 2_000,
            amount_scale: ADA,
        }
    }

    fn test_account() -> account::Config {
        let key = SigningKey::from(hash32(b"test_key"));
        let tag = Tag::from(b"my fav tag".to_vec());
        account::Config { tag, key }
    }

    fn zero() -> Duration {
        Duration::from_secs(0)
    }

    #[test]
    fn scenario_lock_unlock_squash() {
        // One cheque: amount=100, lock@0, unlock 5 ticks later, timeout 10 after unlock, squash 2 after unlock.
        let cheque = Stub::squashed(100, 1, 5, 10, 2);
        let schedule = Schedule::new(&[cheque]);
        println!("schedule");
        println!("{:?}", schedule);

        let mut resolver =
            L2Resolver::new(test_config(), test_account(), schedule, zero()).expect("new fail");
        for _ in 0..3 {
            resolver.tick().expect("tick should apply cleanly");
        }
        assert_eq!(resolver.receipt().lockeds().count(), 1);
        for _ in 3..6 {
            resolver.tick().expect("tick should apply cleanly");
        }
        assert_eq!(resolver.receipt().lockeds().count(), 0);
        assert_eq!(resolver.receipt().unlockeds().count(), 1);
        for _ in 6..10 {
            resolver.tick().expect("tick should apply cleanly");
        }
        assert_eq!(resolver.receipt().unlockeds().count(), 0);
        let body = resolver.receipt().squash().body();
        assert_eq!(body.amount(), 100 * ADA);
        assert_eq!(body.index(), 1);
    }

    #[test]
    fn scenario_two_cheques_concurrent() {
        trace();
        let cheques = vec![
            Stub::squashed(100, 1, 5, 10, 2),
            Stub::unlocked(50, 2, 5, 10),
        ];
        let mut resolver = L2Resolver::new(
            test_config(),
            test_account(),
            Schedule::new(&cheques),
            zero(),
        )
        .expect("new fail");

        for _ in 0..10 {
            resolver.tick().expect("tick should apply cleanly");
        }

        assert_eq!(resolver.receipt().lockeds().count(), 0);
        assert_eq!(resolver.receipt().unlockeds().count(), 1);

        let remaining = resolver
            .receipt()
            .unlockeds()
            .next()
            .expect("cheque 2 should still be unlocked");
        assert_eq!(remaining.index(), 2);
        assert_eq!(remaining.amount(), 50 * ADA);

        let body = resolver.receipt().squash().body();
        assert_eq!(body.amount(), 100 * ADA);
        assert_eq!(body.index(), 1);
    }

    #[test]
    fn scenario_multiple_squash_totals() {
        trace();
        let cheques = vec![
            Stub::squashed(100, 1, 5, 10, 2),
            Stub::squashed(200, 1, 6, 10, 2),
            Stub::squashed(300, 1, 7, 10, 2),
        ];
        let mut resolver = L2Resolver::new(
            test_config(),
            test_account(),
            Schedule::new(&cheques),
            zero(),
        )
        .expect("new fail");

        for _ in 0..12 {
            resolver.tick().expect("tick should apply cleanly");
        }

        assert_eq!(resolver.receipt().lockeds().count(), 0);
        assert_eq!(resolver.receipt().unlockeds().count(), 0);
        assert_eq!(resolver.receipt().squash().body().amount(), 600 * ADA); // 100 + 200 + 300
        assert_eq!(resolver.receipt().squash().body().index(), 3); // max index squashed
    }

    #[test]
    fn scenario_resumability() {
        trace();
        let cheques = vec![
            Stub::squashed(100, 1, 5, 10, 2),
            Stub::unlocked(50, 2, 5, 10),
        ];

        // Straight through, no interruption.
        let mut straight = L2Resolver::new(
            test_config(),
            test_account(),
            Schedule::new(&cheques),
            zero(),
        )
        .expect("new fail");
        for _ in 0..10 {
            straight.tick().expect("tick should apply cleanly");
        }

        // Run halfway, checkpoint, "crash", resume from checkpoint, finish.
        let mut first_half = L2Resolver::new(
            test_config(),
            test_account(),
            Schedule::new(&cheques),
            zero(),
        )
        .expect("new fail");
        for _ in 0..5 {
            first_half.tick().expect("tick should apply cleanly");
        }
        let (checkpoint_tick, checkpoint_zero, checkpoint_receipt) = (
            first_half.tick,
            first_half.zero,
            first_half.receipt().clone(),
        );
        drop(first_half); // simulate crash

        let mut resumed = L2Resolver::resume(
            checkpoint_tick,
            test_config(),
            test_account(),
            Schedule::new(&cheques),
            checkpoint_zero,
            checkpoint_receipt,
        );
        for _ in 5..10 {
            resumed.tick().expect("tick should apply cleanly");
        }

        // Straight-through and resumed runs should land in equivalent end states.
        assert_eq!(
            straight.receipt().lockeds().count(),
            resumed.receipt().lockeds().count()
        );
        assert_eq!(
            straight.receipt().unlockeds().count(),
            resumed.receipt().unlockeds().count()
        );
        assert_eq!(
            straight.receipt().squash().body().amount(),
            resumed.receipt().squash().body().amount()
        );
        assert_eq!(
            straight.receipt().squash().body().index(),
            resumed.receipt().squash().body().index()
        );
    }

    #[test]
    fn resolver_starting_now_anchors_zero_to_now() {
        // starting_now should behave exactly like `new` with zero = now(),
        // bracketed by the actual wall-clock now() calls around it.
        let before = now();
        let resolver = L2Resolver::starting_now(test_config(), test_account(), Schedule::new(&[]))
            .expect("new fail");
        let after = now();

        assert!(
            resolver.zero >= before && resolver.zero <= after,
            "zero ({:?}) should fall between before ({:?}) and after ({:?})",
            resolver.zero,
            before,
            after
        );
        // Sanity: it should behave like any other fresh resolver.
        assert_eq!(resolver.tick, 0);
        assert_eq!(resolver.receipt().lockeds().count(), 0);
    }
}
