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

use konduit_data::{Duration, Locked};
use konduit_rail::{Constraints, Quoted, Relative};
use konduit_tmp::TosInfo;

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
        let timeout = Relative::until(inbound.timeout(), now).saturating_sub(self.time);
        if timeout.is_zero() {
            return Err(Error::InsufficientTime);
        }
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
    use konduit_data::{ChequeBody, Lock, SigningKey, Tag};

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
        let inbound = cheque(10_000, margins.time, now);
        assert_eq!(margins.strip(&inbound, now), Err(Error::InsufficientTime));
    }
}
