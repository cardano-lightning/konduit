use konduit_core::{Duration, Lock, VerifyingKey};

#[cfg(feature = "cbor")]
use minicbor::{Decode, Encode};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct Request {
    #[cfg_attr(feature = "cbor", n(0))]
    pub amount: u64,
}

/// A CLN-specific payment request issued by a recipient.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "cbor", derive(Encode, Decode))]
pub struct Response {
    #[cfg_attr(feature = "cbor", n(0))]
    pub payee: VerifyingKey,
    #[cfg_attr(feature = "cbor", n(1))]
    pub amount: u64,
    #[cfg_attr(feature = "cbor", n(2))]
    pub lock: Lock,
    #[cfg_attr(feature = "cbor", n(3))]
    pub timeout: Duration,
}

#[cfg(test)]
mod tests {
    use super::{Request, Response};
    use konduit_core::{Duration, Lock, VerifyingKey};

    #[cfg(feature = "serde")]
    #[test]
    fn payme_json_roundtrips() {
        let value = sample();
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<Response>(&json).unwrap(), value);
        let request = Request {
            amount: value.amount,
        };
        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), request);
    }

    #[cfg(feature = "cbor")]
    #[test]
    fn payme_cbor_roundtrips() {
        let value = sample();
        let cbor = minicbor::to_vec(&value).unwrap();
        assert_eq!(minicbor::decode::<Response>(&cbor).unwrap(), value);
        let request = Request {
            amount: value.amount,
        };
        let cbor = minicbor::to_vec(&request).unwrap();
        assert_eq!(minicbor::decode::<Request>(&cbor).unwrap(), request);
    }

    fn sample() -> Response {
        Response {
            payee: VerifyingKey::from([1; 32]),
            amount: 2_000_000,
            lock: Lock([2; 32]),
            timeout: Duration::from_secs(300),
        }
    }
}
