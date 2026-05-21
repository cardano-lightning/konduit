/// Functions that generate Aiken test assertions for serialization compatibility.
///
/// Each `*_tests()` function returns a Vec of (test_name, aiken_test_body) pairs.
/// The body is the complete text that goes inside `test <name>() { ... }`.
use crate::{
    Cheque, ChequeBody, Constants, Cont, Datum, Duration, Eol, Indexes, Lock, Locked, Pending,
    Redeemer, Secret, SigningKey, Squash, SquashBody, Stage, Step, Tag, Unlocked, Unpend,
    Unverified, Used, VerifyingKey, aiken::ToAikenLiteral,
};

// ---------------------------------------------------------------------------
// Test vector helpers
// ---------------------------------------------------------------------------

fn encode_hex<T: minicbor::Encode<()>>(val: &T) -> String {
    hex::encode(minicbor::to_vec(val).expect("infallible cbor encode"))
}

fn signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes([seed; 32])
}

fn tag_from(bytes: &[u8]) -> Tag {
    Tag::from(bytes)
}

/// Single encoding assertion rendered as an Aiken test body.
fn encoding_assertion<T>(aiken_lit: &str, rust_val: &T) -> String
where
    T: minicbor::Encode<()>,
{
    format!(
        "  builtin.serialise_data({}) == #\"{}\"\n",
        aiken_lit,
        encode_hex(rust_val)
    )
}

// ---------------------------------------------------------------------------
// Encoding-only tests (no signing)
// ---------------------------------------------------------------------------

pub fn used_tests() -> Vec<(String, String)> {
    let cases: &[(u64, u64)] = &[(0, 0), (1, 1_000_000), (u64::MAX, u64::MAX)];
    cases
        .iter()
        .enumerate()
        .map(|(i, (index, amount))| {
            let val = Used::new(*index, *amount);
            let name = format!("used_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), &val);
            (name, body)
        })
        .collect()
}

pub fn pending_tests() -> Vec<(String, String)> {
    let lock = Lock([0xab; 32]);
    let cases = vec![
        Pending::new(0, Duration::from_millis(0), lock),
        Pending::new(5_000_000, Duration::from_millis(3_600_000), lock),
    ];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("pending_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

pub fn indexes_tests() -> Vec<(String, String)> {
    let cases = vec![
        Indexes::new(vec![]).unwrap(),
        Indexes::new(vec![0, 1, 2]).unwrap(),
        Indexes::new(vec![10, 20, 30]).unwrap(),
    ];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("indexes_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

pub fn cheque_body_tests() -> Vec<(String, String)> {
    let lock = Lock([0xcd; 32]);
    let cases = vec![
        ChequeBody::new(0, 0, Duration::from_millis(0), lock),
        ChequeBody::new(42, 5_000_000, Duration::from_millis(86_400_000), lock),
    ];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("cheque_body_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

pub fn constants_tests() -> Vec<(String, String)> {
    let tag = tag_from(&[0x01, 0x02, 0x03, 0x04]);
    let add_vkey = VerifyingKey::from_bytes([0xaa; 32]);
    let sub_vkey = VerifyingKey::from_bytes([0xbb; 32]);
    let val = Constants {
        tag,
        add_vkey,
        sub_vkey,
        close_period: Duration::from_millis(3_600_000),
    };
    vec![(
        "constants_0".to_string(),
        encoding_assertion(&val.to_aiken_literal(), &val),
    )]
}

pub fn stage_tests() -> Vec<(String, String)> {
    let useds = vec![Used::new(1, 100), Used::new(2, 200)];
    let lock = Lock([0x11; 32]);
    let pendings = vec![Pending::new(500, Duration::from_millis(60_000), lock)];
    let cases = vec![
        Stage::Opened(1_000_000, vec![]),
        Stage::Opened(2_000_000, useds.clone()),
        Stage::Closed(3_000_000, useds.clone(), Duration::from_millis(7_200_000)),
        Stage::Responded(4_000_000, pendings.clone()),
    ];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("stage_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

pub fn datum_tests() -> Vec<(String, String)> {
    let tag = tag_from(&[0xde, 0xad]);
    let add_vkey = VerifyingKey::from_bytes([0xcc; 32]);
    let sub_vkey = VerifyingKey::from_bytes([0xdd; 32]);
    let constants = Constants {
        tag,
        add_vkey,
        sub_vkey,
        close_period: Duration::from_millis(1_800_000),
    };
    let stage = Stage::Opened(0, vec![]);
    let own_hash = [0xeeu8; 28];
    let val = Datum::new(own_hash, constants, stage);
    vec![(
        "datum_0".to_string(),
        encoding_assertion(&val.to_aiken_literal(), &val),
    )]
}

pub fn unpend_tests() -> Vec<(String, String)> {
    let cases = vec![Unpend::Continue, Unpend::Expire, Unpend::Unlock([0x42; 32])];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("unpend_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

pub fn eol_tests() -> Vec<(String, String)> {
    let cases = vec![Eol::End, Eol::Elapse];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("eol_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

pub fn redeemer_tests() -> Vec<(String, String)> {
    let cases = vec![
        Redeemer::Defer,
        Redeemer::Mutual,
        Redeemer::Main(vec![Step::Cont(Cont::Add), Step::Eol(Eol::End)]),
    ];
    cases
        .iter()
        .enumerate()
        .map(|(i, val)| {
            let name = format!("redeemer_{i}");
            let body = encoding_assertion(&val.to_aiken_literal(), val);
            (name, body)
        })
        .collect()
}

/// Locked cheque encoding test (no signing).
pub fn locked_cheque_encoding_tests() -> Vec<(String, String)> {
    let lock = Lock([0xff; 32]);
    let body = ChequeBody::new(1, 2_000_000, Duration::from_millis(60_000), lock);
    let sig_bytes = [0u8; 64]; // dummy signature for encoding-only test
    let sig = crate::Signature::from_bytes(sig_bytes);
    let locked = Locked::<Unverified>::new(body, sig);
    let cheque: Cheque<Unverified> = Cheque::Locked(locked);
    vec![(
        "locked_cheque_encoding_0".to_string(),
        encoding_assertion(&cheque.to_aiken_literal(), &cheque),
    )]
}

/// Unlocked cheque encoding test — EXPECTED TO FAIL due to known structural mismatch.
///
/// Rust encodes `Cheque::Unlocked` as constr 0 + [body_with_secret, sig] (2 fields).
/// Aiken expects `UnlockedCheque(body_with_lock, sig, secret)` (3 fields).
pub fn unlocked_cheque_encoding_tests() -> Vec<(String, String)> {
    let secret = Secret([0x77; 32]);
    let lock = Lock::from(&secret);
    let body_locked = ChequeBody::new(5, 3_000_000, Duration::from_millis(120_000), lock);
    let body_secret = body_locked.try_unlocked(secret).unwrap();
    let sig_bytes = [0u8; 64]; // dummy
    let sig = crate::Signature::from_bytes(sig_bytes);
    let unlocked = Unlocked::<Unverified>::new(body_secret, sig);
    let cheque: Cheque<Unverified> = Cheque::Unlocked(unlocked);
    // The test body notes the expected failure
    let rust_hex = encode_hex(&cheque);
    let aiken_lit = cheque.to_aiken_literal();
    let body = format!(
        "  // EXPECTED TO FAIL: Rust and Aiken use different UnlockedCheque encodings.\n  // Rust: constr0([body_with_secret, sig])  vs  Aiken: constr0([body_with_lock, sig, secret])\n  builtin.serialise_data({}) == #\"{}\"\n",
        aiken_lit, rust_hex
    );
    vec![("unlocked_cheque_encoding_0".to_string(), body)]
}

// ---------------------------------------------------------------------------
// Signing tests (wellsigned)
// ---------------------------------------------------------------------------

fn wellsigned_test_body(
    vk: &VerifyingKey,
    tag: &Tag,
    body_lit: &str,
    sig: &crate::Signature,
) -> String {
    format!(
        "  let vk: VerificationKey = {}\n  let tag: t.Tag = {}\n  let sig = {}\n  wellsigned(vk, tag, {}, sig)\n",
        vk.to_aiken_literal(),
        tag.to_aiken_literal(),
        sig.to_aiken_literal(),
        body_lit
    )
}

pub fn locked_cheque_signing_tests() -> Vec<(String, String)> {
    let sk = signing_key(0);
    let vk = sk.verifying_key();
    let tag = tag_from(&[0xde, 0xad, 0xbe, 0xef]);
    let lock = Lock([0xab; 32]);

    let cases = vec![
        ChequeBody::new(0, 0, Duration::from_millis(0), lock),
        ChequeBody::new(1, 1_000_000, Duration::from_millis(86_400_000), lock),
        ChequeBody::new(42, 5_000_000, Duration::from_millis(3_600_000), lock),
    ];

    cases
        .into_iter()
        .enumerate()
        .map(|(i, body)| {
            let locked = Locked::make(&sk, &tag, body.clone());
            let name = format!("locked_cheque_wellsigned_{i}");
            let body_str = body.to_aiken_literal();
            let test_body = wellsigned_test_body(&vk, &tag, &body_str, &locked.signature);
            (name, test_body)
        })
        .collect()
}

pub fn squash_signing_tests() -> Vec<(String, String)> {
    let sk = signing_key(1);
    let vk = sk.verifying_key();
    let tag = tag_from(&[0x01, 0x02]);

    let cases = vec![
        SquashBody::new_no_verify(0, 0, Indexes::new(vec![]).unwrap()),
        SquashBody::new_no_verify(1_000_000, 5, Indexes::new(vec![1, 2, 3]).unwrap()),
    ];

    cases
        .into_iter()
        .enumerate()
        .map(|(i, body)| {
            let squash = Squash::make(&sk, &tag, body.clone());
            let name = format!("squash_wellsigned_{i}");
            let body_str = body.to_aiken_literal();
            let test_body = wellsigned_test_body(&vk, &tag, &body_str, &squash.signature());
            (name, test_body)
        })
        .collect()
}
