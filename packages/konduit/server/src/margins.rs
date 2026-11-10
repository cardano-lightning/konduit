//! Konduit's fee and time margin, and the only arithmetic that applies
//! them.
//!
//! Konduit charges; a rail does not. A quote leaves padded with
//! konduit's fee and margin on top of the rail's; a commit arrives and
//! has them stripped back off before the rail is called. `strip` is the
//! exact inverse of `pad`, which is what makes a cheque that satisfies a
//! quote necessarily satisfy the rail — so the two live here together,
//! and a caller does neither by hand.
//!
//! This applies to every payment, including one where konduit is itself
//! the payee: being the end of the route is not a discount.

use konduit_core::{Duration, Locked};
use konduit_l2_dto::info::TosInfo;
use konduit_rail::{Constraints, Quoted, Relative};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Margins {
    /// Flat, in the instance asset.
    pub fee: u64,
    /// How much sooner than the inbound cheque the outbound commitment
    /// must resolve, leaving konduit time to act on it.
    pub time: Relative,
}

/// Konduit's own reasons for not committing. The rail is not called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("cheque does not cover konduit's fee")]
    InsufficientFee,
    #[error("cheque does not cover konduit's time margin")]
    InsufficientTime,
}

impl Error {
    pub fn slug(&self) -> &'static str {
        match self {
            Self::InsufficientFee => "insufficient-fee",
            Self::InsufficientTime => "insufficient-time",
        }
    }

    /// Konduit's own problems carry their own status, for the same
    /// reason a rail's do: the enum that defines the problem is what
    /// knows its meaning. The cheque is well formed and cannot be acted
    /// on, so both are 422.
    pub fn status(&self) -> u16 {
        match self {
            Self::InsufficientFee | Self::InsufficientTime => 422,
        }
    }
}

impl Margins {
    pub fn new(fee: u64, time: Relative) -> Self {
        Self { fee, time }
    }

    /// What a consumer is quoted: the rail's terms, padded with
    /// konduit's.
    pub fn pad(&self, quoted: Quoted) -> Quoted {
        Quoted {
            fee: quoted.fee.saturating_add(self.fee),
            timeout: Relative(quoted.timeout.0.saturating_add(self.time.0)),
        }
    }

    /// What the rail is handed: the cheque's terms, less konduit's.
    ///
    /// `amount` is a ceiling and not an instruction — what to pay is the
    /// rail's commit instruction — and `timeout` is relative, from `now`.
    pub fn strip(&self, inbound: &Locked, now: Duration) -> Result<Constraints, Error> {
        let amount = inbound
            .amount()
            .checked_sub(self.fee)
            .ok_or(Error::InsufficientFee)?;
        // Covering the margin exactly is covering it: konduit judges its
        // own margin and nothing else. What is left over may well be too
        // little for a rail to work in, but that is the rail's call —
        // `commit` refuses a zero window itself — and for konduit as the
        // payee there is no onward leg to need any.
        //
        // Checked before subtracting, because `saturating_sub` cannot
        // tell "exactly the margin" from "less than it".
        let remaining = Relative::until(inbound.timeout(), now);
        if remaining < self.time {
            return Err(Error::InsufficientTime);
        }
        let timeout = remaining.saturating_sub(self.time);
        Ok(Constraints {
            amount,
            timeout,
            lock: *inbound.lock(),
        })
    }
}

impl From<&TosInfo> for Margins {
    fn from(tos: &TosInfo) -> Self {
        Self::new(tos.flat_fee, Relative(tos.time_margin))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konduit_core::{ChequeBody, Lock, SigningKey, Tag};

    fn margins() -> Margins {
        Margins::new(1_000, Relative::from_secs(30))
    }

    fn quoted() -> Quoted {
        Quoted {
            fee: 20,
            timeout: Relative::from_secs(60),
        }
    }

    /// A cheque written to exactly what a consumer was quoted.
    fn cheque(amount: u64, timeout: Relative, now: Duration) -> Locked {
        let key = SigningKey::from([1; 32]);
        let body = ChequeBody::new(1, amount, timeout.absolute(now), Lock([2; 32]));
        Locked::make(&key, &Tag::from(b"t".to_vec()), body).into_unverified()
    }

    #[test]
    fn a_cheque_that_satisfies_the_quote_satisfies_the_rail() {
        let (margins, quoted, now) = (margins(), quoted(), Duration::from_secs(1_000));
        let offered = margins.pad(quoted);
        // The consumer pays the quoted amount plus the quoted fee, and
        // allows the quoted timeout, to the millisecond.
        let amount = 5_000 + offered.fee;
        let inbound = cheque(amount, offered.timeout, now);

        let constraints = margins.strip(&inbound, now).expect("covers konduit");
        assert_eq!(
            constraints.amount,
            5_000 + quoted.fee,
            "the rail keeps the whole of what it quoted"
        );
        assert_eq!(
            constraints.timeout, quoted.timeout,
            "and the whole of the time it asked for"
        );
    }

    #[test]
    fn a_cheque_short_of_the_fee_is_konduits_refusal() {
        let margins = margins();
        let now = Duration::from_secs(1_000);
        let inbound = cheque(margins.fee - 1, Relative::from_secs(600), now);
        assert_eq!(margins.strip(&inbound, now), Err(Error::InsufficientFee));
    }

    #[test]
    fn a_cheque_short_of_the_margin_is_konduits_refusal() {
        let margins = margins();
        let now = Duration::from_secs(1_000);
        let short = Relative(margins.time.0.saturating_sub(Duration::from_millis(1)));
        let inbound = cheque(10_000, short, now);
        assert_eq!(margins.strip(&inbound, now), Err(Error::InsufficientTime));
    }

    /// The boundary belongs to konduit's own margin and no further: a
    /// cheque worth exactly the margin covers it, and leaves a rail
    /// nothing — which is konduit's answer, not its refusal. A terminal
    /// payee needs no onward window, and a rail refuses a zero one on
    /// its own.
    #[test]
    fn a_cheque_worth_exactly_the_margin_covers_it() {
        let margins = margins();
        let now = Duration::from_secs(1_000);
        let inbound = cheque(10_000, margins.time, now);
        let constraints = margins.strip(&inbound, now).expect("covers the margin");
        assert!(constraints.timeout.is_zero());
    }
}
