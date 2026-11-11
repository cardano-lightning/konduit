use konduit_core::{
    ChequeBody, Duration, Indexes, Secret, SigningKey, Squash, SquashBody, Tag, Unlocked, Verified,
};

struct SubCase {
    name: &'static str,
    upper_bound: u64,
    useds_in: Vec<(u64, u64)>,
    useds_out: Vec<(u64, u64)>,
    squash: (u64, u64, Vec<u64>),
    unlockeds: Vec<(u64, u64, u64, Secret)>,
    expected: u64,
    expect_failure: bool,
    failure_reason: &'static str,
}

fn aiken_bytes(bytes: &[u8]) -> String {
    format!("#\"{}\"", hex::encode(bytes))
}

fn aiken_useds(useds: &[(u64, u64)]) -> String {
    let useds = useds
        .iter()
        .map(|(index, amount)| format!("({index}, {amount})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{useds}]")
}

fn main() {
    let seed = [0; 32];
    let signing_key = SigningKey::from(seed);
    let verifying_key = signing_key.verifying_key();
    let tag = Tag::from(b"konduit-core-test-vector-v1".as_slice());
    let timeout = 1_800_000_000_000;
    let cases = [
        SubCase {
            name: "test_account_null",
            upper_bound: timeout,
            useds_in: vec![],
            useds_out: vec![],
            squash: (0, 0, vec![]),
            unlockeds: vec![],
            expected: 0,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_account_scenario_0",
            upper_bound: timeout,
            useds_in: vec![(1, 100), (3, 12)],
            useds_out: vec![(1, 100), (2, 100), (3, 12), (4, 12)],
            squash: (0, 0, vec![]),
            unlockeds: vec![
                (2, 100, timeout, Secret([1; 32])),
                (4, 12, timeout, Secret([2; 32])),
            ],
            expected: 224,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_account_scenario_1",
            upper_bound: timeout,
            useds_in: vec![(1, 100), (3, 12)],
            useds_out: vec![(2, 100), (3, 12), (4, 12), (6, 111)],
            squash: (0, 0, vec![]),
            unlockeds: vec![
                (2, 100, timeout, Secret([3; 32])),
                (4, 12, timeout, Secret([4; 32])),
                (6, 111, timeout, Secret([5; 32])),
            ],
            expected: 235,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_ooo_useds_outs",
            upper_bound: timeout,
            useds_in: vec![(1, 100), (3, 12)],
            useds_out: vec![(3, 12), (2, 100), (4, 12), (6, 111)],
            squash: (0, 0, vec![]),
            unlockeds: vec![
                (2, 100, timeout, Secret([6; 32])),
                (4, 12, timeout, Secret([7; 32])),
                (6, 111, timeout, Secret([8; 32])),
            ],
            expected: 235,
            expect_failure: true,
            failure_reason: "useds_out is not strictly increasing",
        },
        SubCase {
            name: "test_ooo_useds_outs_1",
            upper_bound: timeout,
            useds_in: vec![(1, 100), (3, 12)],
            useds_out: vec![(2, 100), (3, 12), (6, 111), (4, 12)],
            squash: (0, 0, vec![]),
            unlockeds: vec![
                (2, 100, timeout, Secret([9; 32])),
                (4, 12, timeout, Secret([10; 32])),
                (6, 111, timeout, Secret([11; 32])),
            ],
            expected: 235,
            expect_failure: true,
            failure_reason: "useds_out is not strictly increasing",
        },
        SubCase {
            name: "test_useds_out_too_long",
            upper_bound: timeout,
            useds_in: vec![(1, 100), (3, 12)],
            useds_out: vec![(2, 100), (3, 12), (4, 12), (6, 111)],
            squash: (0, 0, vec![]),
            unlockeds: vec![
                (2, 100, timeout, Secret([12; 32])),
                (4, 12, timeout, Secret([13; 32])),
            ],
            expected: 235,
            expect_failure: true,
            failure_reason: "missing unlocked for output index 6",
        },
        SubCase {
            name: "test_with_squash",
            upper_bound: timeout,
            useds_in: vec![(1, 100), (3, 12)],
            useds_out: vec![(2, 100), (3, 12), (4, 12)],
            squash: (1_000, 1, vec![]),
            unlockeds: vec![
                (2, 100, timeout, Secret([14; 32])),
                (4, 12, timeout, Secret([15; 32])),
            ],
            expected: 1_124,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_used_already_squashed",
            upper_bound: timeout,
            useds_in: vec![(2, 100), (3, 12)],
            useds_out: vec![(2, 100), (3, 12), (4, 12)],
            squash: (1_000, 2, vec![]),
            unlockeds: vec![(4, 12, timeout, Secret([16; 32]))],
            expected: 1_124,
            expect_failure: true,
            failure_reason: "persisted index is at or below squash index",
        },
        SubCase {
            name: "test_unlocked_timeout_too_early",
            upper_bound: timeout,
            useds_in: vec![],
            useds_out: vec![(1, 5)],
            squash: (0, 0, vec![]),
            unlockeds: vec![(1, 5, timeout - 1, Secret([24; 32]))],
            expected: 5,
            expect_failure: true,
            failure_reason: "unlocked timeout is below upper bound",
        },
        SubCase {
            name: "test_invalid_squash_signature",
            upper_bound: timeout,
            useds_in: vec![],
            useds_out: vec![],
            squash: (0, 0, vec![]),
            unlockeds: vec![],
            expected: 0,
            expect_failure: true,
            failure_reason: "squash signature is invalid",
        },
        SubCase {
            name: "test_invalid_unlocked_signature",
            upper_bound: timeout,
            useds_in: vec![],
            useds_out: vec![(1, 5)],
            squash: (0, 0, vec![]),
            unlockeds: vec![(1, 5, timeout, Secret([27; 32]))],
            expected: 5,
            expect_failure: true,
            failure_reason: "unlocked signature is invalid",
        },
        SubCase {
            name: "test_unlocked_mismatches_output",
            upper_bound: timeout,
            useds_in: vec![],
            useds_out: vec![(1, 5)],
            squash: (0, 0, vec![]),
            unlockeds: vec![(1, 6, timeout, Secret([25; 32]))],
            expected: 5,
            expect_failure: true,
            failure_reason: "unlocked amount does not match output",
        },
        SubCase {
            name: "test_excluded_index_is_skipped",
            upper_bound: timeout,
            useds_in: vec![],
            useds_out: vec![(2, 7)],
            squash: (100, 3, vec![1, 2]),
            unlockeds: vec![(2, 7, timeout, Secret([26; 32]))],
            expected: 107,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_used_not_squashed",
            upper_bound: timeout,
            useds_in: vec![(2, 100)],
            useds_out: vec![(2, 100), (4, 12)],
            squash: (1_000, 3, vec![2]),
            unlockeds: vec![(4, 12, timeout, Secret([17; 32]))],
            expected: 1_112,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_unlocked_already_used",
            upper_bound: timeout,
            useds_in: vec![(2, 100)],
            useds_out: vec![(2, 100)],
            squash: (1_000, 3, vec![2]),
            unlockeds: vec![(2, 100, timeout, Secret([18; 32]))],
            expected: 1_100,
            expect_failure: false,
            failure_reason: "",
        },
        SubCase {
            name: "test_big_scenario",
            upper_bound: timeout,
            useds_in: vec![(81, 1), (82, 1), (83, 1), (85, 1), (86, 1), (87, 1)],
            useds_out: vec![
                (80, 100),
                (82, 1),
                (84, 100),
                (86, 1),
                (88, 100),
                (92, 100),
                (96, 100),
            ],
            squash: (10_000_000, 100, vec![80, 82, 84, 86, 88, 90, 92, 94, 96]),
            unlockeds: vec![
                (80, 100, timeout, Secret([19; 32])),
                (84, 100, timeout, Secret([20; 32])),
                (88, 100, timeout, Secret([21; 32])),
                (92, 100, timeout, Secret([22; 32])),
                (96, 100, timeout, Secret([23; 32])),
            ],
            expected: 10_000_502,
            expect_failure: false,
            failure_reason: "",
        },
    ];

    println!(
        "use aiken/crypto.{{VerificationKey}}\nuse aiken/interval.{{Finite}}\nuse konduit/prelude.{{Amount, Timebound}}\nuse konduit/types as t\n\npub type SubVector {{\n  SubVector {{\n    name: ByteArray,\n    seed: ByteArray,\n    key: VerificationKey,\n    tag: t.Tag,\n    upper_bound: Timebound,\n    useds_in: List<t.Used>,\n    useds_out: List<t.Used>,\n    squash: t.Squash,\n    unlockeds: List<t.Unlocked>,\n    expected: Amount,\n    expected_return: VerificationKey,\n    expect_failure: Bool,\n    failure_reason: ByteArray,\n  }}\n}}\n\npub const vectors: List<SubVector> = ["
    );

    for case in cases {
        let (squash_amount, squash_index, squash_exclude) = &case.squash;
        let squash_body = SquashBody::new(
            *squash_amount,
            *squash_index,
            Indexes::new(squash_exclude.clone()).unwrap(),
        )
        .unwrap();
        let squash = Squash::make(&signing_key, &tag, squash_body);
        let mut squash_signature = squash.signature().to_bytes();
        if case.name == "test_invalid_squash_signature" {
            squash_signature[0] ^= 1;
        }
        assert!(verifying_key.verify(&tag.data(squash.body()), squash.signature()));

        let unlockeds = case
            .unlockeds
            .iter()
            .map(|(index, amount, timeout, secret)| {
                let body =
                    ChequeBody::new(*index, *amount, Duration::from_millis(*timeout), *secret);
                let unlocked = Unlocked::<Verified>::make(&signing_key, &tag, body);
                assert!(
                    verifying_key
                        .verify(&tag.data(&unlocked.body().locked()), unlocked.signature(),)
                );
                format!(
                    "(({}, {}, {}, {}), {})",
                    unlocked.index(),
                    unlocked.amount(),
                    u64::from(unlocked.timeout()),
                    aiken_bytes(unlocked.secret().as_ref()),
                    aiken_bytes(&if case.name == "test_invalid_unlocked_signature" {
                        let mut signature = unlocked.signature().to_bytes();
                        signature[0] ^= 1;
                        signature
                    } else {
                        unlocked.signature().to_bytes()
                    }),
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        println!(
            "  SubVector {{\n    name: \"{}\",\n    seed: {},\n    key: {},\n    tag: {},\n    upper_bound: Finite({}),\n    useds_in: {},\n    useds_out: {},\n    squash: (({}, {}, [{}]), {}),\n    unlockeds: [{}],\n    expected: {},\n    expected_return: {},\n    expect_failure: {},\n    failure_reason: \"{}\",\n  }},",
            case.name,
            aiken_bytes(&seed),
            aiken_bytes(&verifying_key.to_bytes()),
            aiken_bytes(tag.as_ref()),
            case.upper_bound,
            aiken_useds(&case.useds_in),
            aiken_useds(&case.useds_out),
            squash.amount(),
            squash.index(),
            squash_exclude
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            aiken_bytes(&squash_signature),
            unlockeds,
            case.expected,
            aiken_bytes(&verifying_key.to_bytes()),
            if case.expect_failure { "True" } else { "False" },
            case.failure_reason,
        );
    }

    println!("]");
}
