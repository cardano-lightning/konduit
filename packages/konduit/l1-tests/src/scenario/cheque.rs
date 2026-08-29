use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

static SEPARATOR: &str = " ";

/// A cheque stub.
/// This is subtle, but important!
/// If three fields: amount, lock, timeout; timeout relative to lock.
/// If four fields: amount, lock, unlock, timeout; timeout relative to unlock.
/// If five fields: amount, lock, unlock, timeout, squash; timeout and squash relative to unlock.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Stub {
    /// Amount (ada ie 1_000_000 x)
    pub amount: u64,
    /// Issue locked cheque at tick `lock`
    pub lock: usize,
    /// Unlock cheque at `unlock` ticks after issuance.
    /// If `unlock == None`, then no resolution.
    pub unlock: Option<usize>,
    /// Timeout: relative to `lock` if `unlock == None`, otherwise relative to `unlock`. {};
    pub timeout: usize,
    /// Squash `squash` ticks after `unlock`. Ignored if `unlock == None`. If `squash == None`, then no squash.
    pub squash: Option<usize>,
}

impl Stub {
    pub fn new(
        amount: u64,
        lock: usize,
        unlock: Option<usize>,
        timeout: usize,
        squash: Option<usize>,
    ) -> Self {
        Self {
            amount,
            lock,
            unlock,
            timeout,
            squash,
        }
    }

    pub fn locked(amount: u64, lock: usize, timeout: usize) -> Self {
        Self::new(amount, lock, None, timeout, None)
    }

    pub fn unlocked(amount: u64, lock: usize, unlock: usize, timeout: usize) -> Self {
        Self::new(amount, lock, Some(unlock), timeout, None)
    }

    pub fn squashed(
        amount: u64,
        lock: usize,
        unlock: usize,
        timeout: usize,
        squash: usize,
    ) -> Self {
        Self::new(amount, lock, Some(unlock), timeout, Some(squash))
    }
}

impl TryFrom<String> for Stub {
    type Error = ParseChequeError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl From<Stub> for String {
    fn from(c: Stub) -> Self {
        c.to_string()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseChequeError {
    #[error("too few fields")]
    TooFew,
    #[error("too many fields")]
    TooMany,
    #[error("bad field: {0}")]
    BadField(&'static str),
}

impl FromStr for Stub {
    type Err = ParseChequeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ParseChequeError::*;

        fn field<T: FromStr>(s: &str, name: &'static str) -> Result<T, ParseChequeError> {
            s.parse().map_err(|_| BadField(name))
        }

        let parts: Vec<&str> = s.split(SEPARATOR).map(str::trim).collect();
        match parts.as_slice() {
            [a, l, t] => Ok(Stub::locked(
                field(a, "amount")?,
                field(l, "lock")?,
                field(t, "timeout")?,
            )),
            [a, l, u, t] => Ok(Stub::unlocked(
                field(a, "amount")?,
                field(l, "lock")?,
                field(u, "unlock")?,
                field(t, "timeout")?,
            )),
            [a, l, u, t, sq] => Ok(Stub::squashed(
                field(a, "amount")?,
                field(l, "lock")?,
                field(u, "unlock")?,
                field(t, "timeout")?,
                field(sq, "squash")?,
            )),
            [] | [_] | [_, _] => Err(TooFew),
            _ => Err(TooMany),
        }
    }
}

impl fmt::Display for Stub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut fields = vec![self.amount.to_string(), self.lock.to_string()];
        match self.unlock {
            Some(u) => {
                fields.push(u.to_string());
                fields.push(self.timeout.to_string());
                if let Some(s) = self.squash {
                    fields.push(s.to_string());
                }
            }
            None => {
                fields.push(self.timeout.to_string());
            }
        }
        write!(f, "{}", fields.join(SEPARATOR))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_squashed() {
        let c: Stub = "50 0 1 2 3".parse().unwrap();
        assert_eq!(c, Stub::squashed(50, 0, 1, 2, 3));
    }

    #[test]
    fn parses_unlocked_no_squash() {
        let c: Stub = "50 0 1 2".parse().unwrap();
        assert_eq!(c, Stub::unlocked(50, 0, 1, 2));
    }

    #[test]
    fn parses_locked_no_unlock() {
        let c: Stub = "20 1 5".parse().unwrap();
        assert_eq!(c, Stub::locked(20, 1, 5));
    }

    #[test]
    fn rejects_too_few() {
        assert!(matches!(
            "50".parse::<Stub>(),
            Err(ParseChequeError::TooFew)
        ));
        assert!(matches!(
            "50 0".parse::<Stub>(),
            Err(ParseChequeError::TooFew)
        ));
        assert!(matches!("".parse::<Stub>(), Err(ParseChequeError::TooFew)));
    }

    #[test]
    fn rejects_too_many() {
        assert!(matches!(
            "50 0 1 2 3 9".parse::<Stub>(),
            Err(ParseChequeError::TooMany)
        ));
    }

    #[test]
    fn rejects_bad_field() {
        assert!(matches!(
            "x 0 5".parse::<Stub>(),
            Err(ParseChequeError::BadField("amount"))
        ));
        assert!(matches!(
            "50 0 x".parse::<Stub>(),
            Err(ParseChequeError::BadField("timeout"))
        ));
        assert!(matches!(
            "50 0 x 2".parse::<Stub>(),
            Err(ParseChequeError::BadField("unlock"))
        ));
        assert!(matches!(
            "50 0 1 2 x".parse::<Stub>(),
            Err(ParseChequeError::BadField("squash"))
        ));
    }

    #[test]
    fn display_locked() {
        let c = Stub::locked(20, 1, 5);
        assert_eq!(c.to_string(), "20 1 5");
    }

    #[test]
    fn display_unlocked_no_squash() {
        let c = Stub::unlocked(50, 0, 1, 2);
        assert_eq!(c.to_string(), "50 0 1 2");
    }

    #[test]
    fn display_squashed() {
        let c = Stub::squashed(50, 0, 1, 2, 3);
        assert_eq!(c.to_string(), "50 0 1 2 3");
    }
}
