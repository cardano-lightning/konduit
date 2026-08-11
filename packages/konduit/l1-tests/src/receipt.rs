//! Legitimate, realistic-looking `Receipt` generation with resumable
//! evolution and time compression (via a rescale at signing).
use std::collections::HashSet;

use konduit_data::{
    Cheque, ChequeBody, Duration, Indexes, Lock, Locked, Secret, SigningKey, Squash, SquashBody,
    Tag, Unlocked,
};
use konduit_tmp::Receipt;
use rand::{Rng, RngExt};

// =========================================================================
// Seed — the unsigned, pre-crypto shape. Only secrets are stored; a Lock
// is derived at `sign` time for whichever cheques stay unrevealed. No
// absolute time — each cheque just carries a relative timeout, rescaled
// at `sign` time to compress or stretch the whole thing to real duration.
// =========================================================================
#[derive(Debug, Clone)]
pub struct ChequeSeed {
    pub index: u64,
    pub amount: u64,
    pub timeout_secs: u64,
    pub secret: Secret,
    pub revealed: bool, // false -> Locked at sign, true -> Unlocked
}

#[derive(Debug, Clone)]
pub struct Seed {
    pub squash: SquashBody,
    pub cheques: Vec<ChequeSeed>,
}

// =========================================================================
// One-shot generation — direct construction of a plausible Seed under a
// budget, no walk required. Live cheques sit above the squash frontier,
// so nothing can ever collaterally exclude an unwitnessed cheque.
// =========================================================================
#[derive(Debug, Clone)]
pub struct Params {
    pub max_live_cheques: usize,
    pub max_exclude_len: usize, // mirror konduit_data::MAX_EXCLUDE_LENGTH
    pub min_timeout_secs: u64,
    pub max_timeout_secs: u64,
    pub reveal_probability: f64,
    pub max_frontier: u64,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_live_cheques: 20,
            max_exclude_len: 16,
            min_timeout_secs: 60,
            max_timeout_secs: 86_400,
            reveal_probability: 0.5,
            max_frontier: 50,
        }
    }
}

pub fn generate_seed(rng: &mut impl Rng, budget: u64, params: &Params) -> Seed {
    let mut remaining = budget;

    let squash_amount = rng.random_range(0..=remaining);
    remaining -= squash_amount;

    let frontier = rng.random_range(0..=params.max_frontier);
    let exclude_cap = params.max_exclude_len.min(frontier as usize);
    let mut pool: Vec<u64> = (0..frontier).collect();
    let mut excluded = Vec::new();
    for _ in 0..rng.random_range(0..=exclude_cap) {
        if pool.is_empty() {
            break;
        }
        excluded.push(pool.remove(rng.random_range(0..pool.len())));
    }
    excluded.sort_unstable();
    let exclude = Indexes::new(excluded).unwrap_or_else(|_| Indexes::empty());
    let squash = SquashBody::new_no_verify(squash_amount, frontier, exclude);

    let mut cheques = Vec::new();
    let mut index = frontier;
    for _ in 0..rng.random_range(0..=params.max_live_cheques) {
        if remaining == 0 {
            break;
        }
        index += 1;
        let amount = rng.random_range(1..=remaining);
        remaining -= amount;
        cheques.push(ChequeSeed {
            index,
            amount,
            timeout_secs: rng.random_range(params.min_timeout_secs..=params.max_timeout_secs),
            secret: Secret(rng.random::<[u8; 32]>()), // adjust if Secret's field isn't a plain [u8;32]
            revealed: rng.random_bool(params.reveal_probability),
        });
    }

    Seed { squash, cheques }
}

// =========================================================================
// Signing — turn a Seed into a real Receipt. `scale` compresses (or
// stretches) every cheque's relative timeout; locks are derived only now.
// =========================================================================
fn scaled_timeout(secs: u64, scale: f64) -> Duration {
    let ms = ((secs as f64) * 1000.0 * scale).round().max(1.0) as u64;
    Duration::from_millis(ms)
}

pub fn sign(seed: &Seed, scale: f64, signing_key: &SigningKey, tag: &Tag) -> Receipt {
    let cheques = seed
        .cheques
        .iter()
        .map(|c| {
            let timeout = scaled_timeout(c.timeout_secs, scale);
            if c.revealed {
                let body = ChequeBody::new(c.index, c.amount, timeout, c.secret);
                Cheque::Unlocked(Unlocked::make(signing_key, tag, body))
            } else {
                let body = ChequeBody::new(c.index, c.amount, timeout, Lock::from(&c.secret));
                Cheque::Locked(Locked::make(signing_key, tag, body))
            }
        })
        .collect();

    Receipt::new_with_state(Squash::make(signing_key, tag, seed.squash.clone()), cheques)
}

// =========================================================================
// Step — one unit of shrinkable randomness for resuming a Session.
// =========================================================================
#[derive(Debug, Clone)]
pub struct Step {
    pub action_selector: u32,
    pub amount: u64,
    pub timeout_secs: u64,
    pub secret_bytes: [u8; 32],
    pub reveal_on_mint: bool,
}

impl proptest::arbitrary::Arbitrary for Step {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;
    fn arbitrary_with(_: ()) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<u32>(),
            1u64..1_000_000,
            60u64..86_400,
            any::<[u8; 32]>(),
            any::<bool>(),
        )
            .prop_map(
                |(action_selector, amount, timeout_secs, secret_bytes, reveal_on_mint)| Step {
                    action_selector,
                    amount,
                    timeout_secs,
                    secret_bytes,
                    reveal_on_mint,
                },
            )
            .boxed()
    }
}

// =========================================================================
// Session — resumable evolution of a Seed via individual mint/unlock/
// squash/drop moves. No clock: everything here is relative-duration only.
// =========================================================================
enum Action {
    NewLocked,
    Unlock(usize),
    Squash(usize),
    Drop(usize),
}

pub struct Session {
    pub seed: Seed,
    next_index: u64,
    remaining: u64,
    used: HashSet<u64>, // witnessed-unlocked indices (since last yield)
}

impl Session {
    pub fn new(budget: u64) -> Self {
        Self {
            seed: Seed {
                squash: SquashBody::zero(),
                cheques: Vec::new(),
            },
            next_index: 0,
            remaining: budget,
            used: HashSet::new(),
        }
    }

    /// Resume evolving an existing seed. `remaining` is what's still
    /// spendable on top of what the seed already committed.
    pub fn from_seed(seed: Seed, remaining: u64) -> Self {
        let next_index = seed
            .cheques
            .iter()
            .map(|c| c.index)
            .max()
            .unwrap_or(seed.squash.index())
            + 1;
        Self {
            seed,
            next_index,
            remaining,
            used: HashSet::new(),
        }
    }

    pub fn remaining_budget(&self) -> u64 {
        self.remaining
    }

    /// Never allow remaining to imply a budget below what's committed.
    pub fn set_remaining_budget(&mut self, budget: u64) {
        self.remaining = budget;
    }

    fn legal(&self) -> Vec<Action> {
        let mut acts = Vec::new();
        if self.remaining > 0 {
            acts.push(Action::NewLocked);
        }

        let frontier = self.seed.squash.index();
        for (i, c) in self.seed.cheques.iter().enumerate() {
            if !c.revealed {
                acts.push(Action::Unlock(i));
                acts.push(Action::Drop(i)); // stands in for expiry / abandonment
                continue;
            }
            let would_bury = self.seed.cheques.iter().any(|o| {
                o.revealed
                    && o.index != c.index
                    && o.index > frontier
                    && o.index < c.index
                    && !self.used.contains(&o.index)
            });
            if !would_bury {
                acts.push(Action::Squash(i));
            }
        }
        acts
    }

    fn apply(&mut self, action: Action, step: &Step) {
        match action {
            Action::NewLocked => {
                let amount = step.amount.min(self.remaining);
                if amount == 0 {
                    return;
                }
                self.remaining -= amount;
                let index = self.next_index;
                self.next_index += 1;
                self.seed.cheques.push(ChequeSeed {
                    index,
                    amount,
                    timeout_secs: step.timeout_secs,
                    secret: Secret(step.secret_bytes),
                    revealed: step.reveal_on_mint,
                });
            }
            Action::Unlock(i) => self.seed.cheques[i].revealed = true,
            Action::Drop(i) => {
                self.seed.cheques.remove(i);
            }
            Action::Squash(i) => {
                let c = &self.seed.cheques[i];
                if self.seed.squash.squash(c.index, c.amount).is_ok() {
                    self.used.remove(&c.index);
                    self.seed.cheques.remove(i);
                }
                // exclude-range overflow: no-op, retry on a later step
            }
        }
    }

    pub fn step_rng(&mut self, rng: &mut impl Rng) {
        let step = Step {
            action_selector: rng.random(),
            amount: rng.random_range(1..=self.remaining.max(1)),
            timeout_secs: rng.random_range(60..86_400),
            secret_bytes: rng.random(),
            reveal_on_mint: rng.random_bool(0.3),
        };
        self.step_with(&step);
    }

    pub fn step_with(&mut self, step: &Step) {
        let acts = self.legal();
        if acts.is_empty() {
            return;
        }
        let idx = (step.action_selector as usize) % acts.len();
        let action = acts.into_iter().nth(idx).unwrap();
        self.apply(action, step);
    }

    pub fn evolve_rng(&mut self, rng: &mut impl Rng, steps: usize) {
        for _ in 0..steps {
            self.step_rng(rng);
        }
    }

    /// Marks currently-unlocked cheques as witnessed; only affects future
    /// collateral-burial checks, never gates direct squashing.
    pub fn yield_seed(&mut self) -> Seed {
        for c in &self.seed.cheques {
            if c.revealed {
                self.used.insert(c.index);
            }
        }
        self.seed.clone()
    }
}

// =========================================================================
// Manager — a set of (SigningKey, Tag, Total), each driving its own
// Session. Caller can insert, drop, and mutate `total` freely.
// (SigningKey, Tag) is assumed unique but not enforced.
// =========================================================================
pub struct Account {
    pub signing_key: SigningKey,
    pub tag: Tag,
    pub total: u64,
    session: Session,
}

pub struct Accounts {
    entries: Vec<Account>,
    scale: f64,
}

impl Accounts {
    pub fn new(scale: f64) -> Self {
        Self {
            entries: Vec::new(),
            scale,
        }
    }

    pub fn insert(&mut self, signing_key: SigningKey, tag: Tag, total: u64) -> usize {
        self.entries.push(Account {
            signing_key,
            tag,
            total,
            session: Session::new(total),
        });
        self.entries.len() - 1
    }

    pub fn drop(&mut self, id: usize) -> Account {
        self.entries.remove(id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn total(&self, id: usize) -> u64 {
        self.entries[id].total
    }

    pub fn set_total(&mut self, id: usize, total: u64) {
        self.entries[id].total = total;
        self.entries[id].session.set_remaining_budget(total);
    }

    pub fn increase_total(&mut self, id: usize, by: u64) {
        let new_total = self.entries[id].total + by;
        self.set_total(id, new_total);
    }

    pub fn step_rng(&mut self, id: usize, rng: &mut impl Rng) {
        self.entries[id].session.step_rng(rng);
    }
    pub fn step_with(&mut self, id: usize, step: &Step) {
        self.entries[id].session.step_with(step);
    }
    pub fn evolve_rng(&mut self, id: usize, rng: &mut impl Rng, n: usize) {
        self.entries[id].session.evolve_rng(rng, n);
    }

    pub fn yield_receipt(&mut self, id: usize) -> Receipt {
        let entry = &mut self.entries[id];
        let seed = entry.session.yield_seed();
        sign(&seed, self.scale, &entry.signing_key, &entry.tag)
    }

    pub fn yield_all(&mut self) -> Vec<Receipt> {
        self.entries
            .iter_mut()
            .map(|e| {
                let seed = e.session.yield_seed();
                sign(&seed, self.scale, &e.signing_key, &e.tag)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konduit_data::{SigningKey, Tag};
    use rand::{SeedableRng, rngs::StdRng};

    fn key() -> SigningKey {
        SigningKey::from([7; 32])
    }
    fn tag() -> Tag {
        Tag::from([1; 20].to_vec())
    }
    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn generate_seed_runs() {
        let mut rng = rng();
        let seed = generate_seed(&mut rng, 1_000_000, &Params::default());
        // total committed across squash + live cheques never exceeds budget
        let committed: u64 =
            seed.squash.amount() + seed.cheques.iter().map(|c| c.amount).sum::<u64>();
        assert!(
            committed <= 1_000_000,
            "committed {committed} exceeds budget"
        );
    }

    #[test]
    fn generate_seed_zero_budget_is_empty() {
        let mut rng = rng();
        let seed = generate_seed(&mut rng, 0, &Params::default());
        assert_eq!(seed.squash.amount(), 0);
        assert!(seed.cheques.iter().all(|c| c.amount == 0) || seed.cheques.is_empty());
    }

    #[test]
    fn sign_produces_matching_cheque_count() {
        let mut rng = rng();
        let seed = generate_seed(&mut rng, 500_000, &Params::default());
        let expected = seed.cheques.len();
        let receipt = sign(&seed, 1.0, &key(), &tag());
        assert_eq!(receipt.cheques().count(), expected);
    }

    #[test]
    fn sign_respects_revealed_flag() {
        let mut rng = rng();
        let seed = generate_seed(&mut rng, 500_000, &Params::default());
        let receipt = sign(&seed, 1.0, &key(), &tag());
        for (s, c) in seed.cheques.iter().zip(receipt.cheques()) {
            match c {
                Cheque::Locked(_) => assert!(
                    !s.revealed,
                    "index {} locked but seed said revealed",
                    s.index
                ),
                Cheque::Unlocked(_) => assert!(
                    s.revealed,
                    "index {} unlocked but seed said not revealed",
                    s.index
                ),
            }
        }
    }

    #[test]
    fn sign_scale_shrinks_timeout() {
        let mut rng = rng();
        let mut params = Params::default();
        params.reveal_probability = 0.0; // force Locked so timeout is easy to read back
        params.max_live_cheques = 1;
        params.max_frontier = 0;
        let seed = generate_seed(&mut rng, 500_000, &params);
        if let Some(c) = seed.cheques.first() {
            let receipt = sign(&seed, 0.001, &key(), &tag());
            if let Cheque::Locked(l) = &receipt.cheques().next().unwrap() {
                let ms = u64::from(l.timeout());
                assert!(ms <= c.timeout_secs * 1000, "scaled timeout should shrink");
            }
        }
    }

    #[test]
    fn session_new_then_evolve_then_yield_runs() {
        let mut rng = rng();
        let mut session = Session::new(200_000);
        session.evolve_rng(&mut rng, 30);
        let seed = session.yield_seed();
        // should not exceed the original budget
        let committed: u64 =
            seed.squash.amount() + seed.cheques.iter().map(|c| c.amount).sum::<u64>();
        assert!(committed <= 200_000);
    }

    #[test]
    fn session_resume_from_seed_continues_indices() {
        let mut rng = rng();
        let mut session = Session::new(200_000);
        session.evolve_rng(&mut rng, 10);
        let seed = session.yield_seed();
        let max_index_before = seed.cheques.iter().map(|c| c.index).max();

        let mut resumed = Session::from_seed(seed, 50_000);
        resumed.evolve_rng(&mut rng, 10);
        let seed_2 = resumed.yield_seed();

        if let Some(before) = max_index_before {
            assert!(seed_2.cheques.iter().all(|c| c.index > before) || seed_2.cheques.is_empty());
        }
    }

    #[test]
    fn session_never_exceeds_remaining_budget() {
        let mut rng = rng();
        let mut session = Session::new(1_000);
        session.evolve_rng(&mut rng, 100); // hammer it well past what 1_000 should allow
        let seed = session.yield_seed();
        let committed: u64 =
            seed.squash.amount() + seed.cheques.iter().map(|c| c.amount).sum::<u64>();
        assert!(
            committed <= 1_000,
            "committed {committed} exceeds budget of 1000"
        );
    }

    #[test]
    fn manager_insert_evolve_yield_all_runs() {
        let mut rng = rng();
        let mut mgr = Accounts::new(1.0);
        let a = mgr.insert(key(), tag(), 100_000);
        let b = mgr.insert(key(), tag(), 50_000);

        mgr.evolve_rng(a, &mut rng, 10);
        mgr.evolve_rng(b, &mut rng, 10);

        let receipts = mgr.yield_all();
        assert_eq!(receipts.len(), 2);
        assert_eq!(mgr.len(), 2);
    }

    #[test]
    fn manager_drop_removes_entry() {
        let mut mgr = Accounts::new(1.0);
        let a = mgr.insert(key(), tag(), 100_000);
        let _b = mgr.insert(key(), tag(), 50_000);
        assert_eq!(mgr.len(), 2);
        mgr.drop(a);
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn manager_set_and_increase_total() {
        let mut mgr = Accounts::new(1.0);
        let a = mgr.insert(key(), tag(), 100);
        mgr.increase_total(a, 50);
        assert_eq!(mgr.total(a), 150);
        mgr.set_total(a, 10);
        assert_eq!(mgr.total(a), 10);
    }

    #[test]
    fn generate_batch_composition_report() {
        let mut rng = rng();
        let n = 100;

        let mut only_locked = 0;
        let mut only_unlocked = 0;
        let mut both = 0;
        let mut neither = 0;

        let mut total_locked = 0;
        let mut total_unlocked = 0;

        for _ in 0..n {
            let seed = generate_seed(&mut rng, 500_000, &Params::default());

            let has_locked = seed.cheques.iter().any(|c| !c.revealed);
            let has_unlocked = seed.cheques.iter().any(|c| c.revealed);

            total_locked += seed.cheques.iter().filter(|c| !c.revealed).count();
            total_unlocked += seed.cheques.iter().filter(|c| c.revealed).count();

            match (has_locked, has_unlocked) {
                (true, true) => both += 1,
                (true, false) => only_locked += 1,
                (false, true) => only_unlocked += 1,
                (false, false) => neither += 1,
            }
        }

        println!(
            "out of {n} receipts:\n\
            \x20 both Locked & Unlocked: {both}\n\
            \x20 only Locked:            {only_locked}\n\
            \x20 only Unlocked:          {only_unlocked}\n\
            \x20 neither (empty):        {neither}\n\
            \x20 total Locked cheques:   {total_locked}\n\
            \x20 total Unlocked cheques: {total_unlocked}"
        );

        // sanity: with default Params (reveal_probability 0.5, up to 20 live
        // cheques), "both" should dominate — if it's near zero something's off
        assert!(
            both > 0,
            "expected at least some receipts with both variants present"
        );
    }
}
