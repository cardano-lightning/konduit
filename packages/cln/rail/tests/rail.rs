//! The cln rail's answers that need no network: what it says about a
//! payee it cannot reach, constraints it cannot work within, and a lock
//! it knows nothing about.

use cln_rail::{Channels, Commits, Ctx, Payee, Paymes, Signer, paymes, signer};
use konduit_data::{Duration, Lock, SigningKey};
use konduit_rail::{Constraints, Outcome, Rail, Registry, Relative, Resolution};

/// A rail with no outbound peers at all.
fn rail() -> Ctx {
    Ctx::new(
        Paymes::new(paymes::Config::default()),
        Channels::new(),
        Commits::new(),
        Signer::new(signer::Config { key: [7; 32] }),
    )
}

fn payee() -> Payee {
    Payee(SigningKey::from([9; 32]).verifying_key())
}

fn constraints(timeout: Relative) -> Constraints {
    Constraints {
        amount: 1_000,
        timeout,
        lock: Lock([1; 32]),
    }
}

#[tokio::test]
async fn a_payee_with_no_channel_is_unroutable() {
    let outcome = rail()
        .commit(constraints(Relative::from_secs(60)), payee())
        .await;
    match outcome {
        Outcome::FailedPre(problem) => assert_eq!(problem.slug(), "unroutable"),
        other => panic!("expected FailedPre, got {other:?}"),
    }
}

#[tokio::test]
async fn no_time_left_is_insufficient_margin() {
    // Konduit's margin has eaten the whole timeout, so there is nothing
    // to commit within. The rail keeps no margin of its own, but it
    // still cannot write a cheque that has already elapsed.
    let outcome = rail()
        .commit(constraints(Relative::from_secs(0)), payee())
        .await;
    match outcome {
        Outcome::FailedPre(problem) => assert_eq!(problem.slug(), "insufficient-margin"),
        other => panic!("expected FailedPre, got {other:?}"),
    }
}

#[tokio::test]
async fn an_unroutable_quote_is_not_a_rail_fault() {
    let problem = rail()
        .quote(1_000, payee())
        .await
        .expect_err("no channel to quote over");
    assert_eq!(problem.slug(), "unroutable");
}

#[tokio::test]
async fn a_lock_never_committed_is_unknown() {
    assert_eq!(rail().reveal(&Lock([2; 32])).await, Resolution::Unknown);
}

#[tokio::test]
async fn the_rail_registers_as_cln() {
    let mut registry = Registry::new();
    registry.insert(rail()).expect("cln is not reserved");
    assert_eq!(registry.names(), vec!["cln"]);

    // And is reachable with its target erased to bytes, as the platform
    // will hand it over.
    let target = konduit_rail::RawTarget::Json(serde_json::to_vec(&payee()).unwrap());
    let outcome = registry
        .get("cln")
        .unwrap()
        .commit(constraints(Relative::from_secs(60)), &target)
        .await;
    match outcome {
        Outcome::FailedPre(problem) => assert_eq!(problem.slug(), "unroutable"),
        other => panic!("expected FailedPre, got {other:?}"),
    }
}

#[test]
fn a_secret_is_cached_as_long_as_the_commitment_it_unlocked() {
    use cln_rail::commits::{Claim, Commit};
    use konduit_data::Secret;

    let commits = Commits::new();
    let lock = Lock([3; 32]);
    let timeout = Duration::from_secs(100);
    assert!(matches!(
        commits.claim(lock, Commit::new(1, 1_000, timeout)),
        Claim::Fresh
    ));

    // A repeat under the one lock replays the first outcome rather than
    // paying again or erroring.
    assert!(matches!(
        commits.claim(lock, Commit::new(2, 1_000, timeout)),
        Claim::Existing(Outcome::Pending)
    ));

    let secret = Secret([4; 32]);
    commits.resolve(&lock, secret).unwrap();
    assert!(matches!(
        commits.claim(lock, Commit::new(2, 1_000, timeout)),
        Claim::Existing(Outcome::Success(s)) if s == secret
    ));

    commits.prune(Duration::from_secs(99));
    assert!(commits.get(&lock).is_some(), "cached until the timeout");
    commits.prune(Duration::from_secs(100));
    assert!(commits.get(&lock).is_none(), "and no longer");
}

#[test]
fn giving_up_a_claim_frees_the_lock() {
    use cln_rail::commits::{Claim, Commit};

    let commits = Commits::new();
    let lock = Lock([5; 32]);
    let commit = Commit::new(1, 1_000, Duration::from_secs(100));
    assert!(matches!(commits.claim(lock, commit.clone()), Claim::Fresh));
    commits.release(&lock);
    assert!(
        matches!(commits.claim(lock, commit), Claim::Fresh),
        "nothing was sent, so a retry is free to proceed"
    );
}
