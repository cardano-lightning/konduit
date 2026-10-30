use konduit_data::{Duration, Lock, VerifyingKey};
use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct PaymeRequest {
    #[n(0)]
    pub amount: u64,
}

/// A CLN-network payment request created by the recipient.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Payme {
    #[n(0)]
    pub payee: VerifyingKey,
    #[n(1)]
    pub amount: u64,
    #[n(2)]
    pub lock: Lock,
    #[n(3)]
    pub timeout: Duration,
}

#[cfg(test)]
mod tests {
    use super::Payme;
    use konduit_data::{Duration, Lock, VerifyingKey};

    #[test]
    fn payme_roundtrips_json_and_cbor() {
        let payme = Payme {
            payee: VerifyingKey::from([1; 32]),
            amount: 2_000_000,
            lock: Lock([2; 32]),
            timeout: Duration::from_secs(300),
        };

        let json = serde_json::to_string(&payme).expect("payme serializes to JSON");
        assert_eq!(serde_json::from_str::<Payme>(&json).unwrap(), payme);

        let cbor = minicbor::to_vec(&payme).expect("payme serializes to CBOR");
        assert_eq!(minicbor::decode::<Payme>(&cbor).unwrap(), payme);
    }
}
