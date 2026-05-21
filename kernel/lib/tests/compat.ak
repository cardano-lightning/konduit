// GENERATED — do not edit by hand.
// Re-run: cargo run --features aiken -p konduit-data --bin generate_compat_tests
//
// Encoding tests: builtin.serialise_data(<aiken_value>) == #"<rust_cbor_hex>"
// Signing tests:  wellsigned(vk, tag, <body>, sig)

use aiken/builtin
use aiken/crypto.{VerificationKey}
use konduit/types as t
use konduit/wellformed.{wellsigned}

test used_0() {
  builtin.serialise_data((0, 0)) == #"9f0000ff"
}

test used_1() {
  builtin.serialise_data((1, 1000000)) == #"9f011a000f4240ff"
}

test used_2() {
  builtin.serialise_data((18446744073709551615, 18446744073709551615)) == #"9f1bffffffffffffffff1bffffffffffffffffff"
}

test pending_0() {
  builtin.serialise_data(
    (0, 0, #"abababababababababababababababababababababababababababababababab"),
  ) == #"9f00005820ababababababababababababababababababababababababababababababababff"
}

test pending_1() {
  builtin.serialise_data(
    (
      5000000,
      3600000,
      #"abababababababababababababababababababababababababababababababab",
    ),
  ) == #"9f1a004c4b401a0036ee805820ababababababababababababababababababababababababababababababababff"
}

test indexes_0() {
  builtin.serialise_data([]) == #"80"
}

test indexes_1() {
  builtin.serialise_data([0, 1, 2]) == #"9f000102ff"
}

test indexes_2() {
  builtin.serialise_data([10, 20, 30]) == #"9f0a14181eff"
}

test cheque_body_0() {
  builtin.serialise_data(
    (
      0,
      0,
      0,
      #"cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
    ),
  ) == #"9f0000005820cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdff"
}

test cheque_body_1() {
  builtin.serialise_data(
    (
      42,
      5000000,
      86400000,
      #"cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd",
    ),
  ) == #"9f182a1a004c4b401a05265c005820cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdff"
}

test constants_0() {
  builtin.serialise_data(
    t.Constants {
      tag: #"01020304",
      add_vkey: #"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      sub_vkey: #"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      close_period: 3600000,
    },
  ) == #"d8799f44010203045820aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa5820bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb1a0036ee80ff"
}

test stage_0() {
  builtin.serialise_data(t.Opened(1000000, [])) == #"d8799f1a000f424080ff"
}

test stage_1() {
  builtin.serialise_data(t.Opened(2000000, [(1, 100), (2, 200)])) == #"d8799f1a001e84809f9f011864ff9f0218c8ffffff"
}

test stage_2() {
  builtin.serialise_data(t.Closed(3000000, [(1, 100), (2, 200)], 7200000)) == #"d87a9f1a002dc6c09f9f011864ff9f0218c8ffff1a006ddd00ff"
}

test stage_3() {
  builtin.serialise_data(
    t.Responded(
      4000000,
      [
        (
          500,
          60000,
          #"1111111111111111111111111111111111111111111111111111111111111111",
        ),
      ],
    ),
  ) == #"d87b9f1a003d09009f9f1901f419ea6058201111111111111111111111111111111111111111111111111111111111111111ffffff"
}

test datum_0() {
  builtin.serialise_data(
    (
      #"eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      t.Constants {
        tag: #"dead",
        add_vkey: #"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        sub_vkey: #"dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        close_period: 1800000,
      },
      t.Opened(0, []),
    ),
  ) == #"9f581ceeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeed8799f42dead5820cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc5820dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd1a001b7740ffd8799f0080ffff"
}

test unpend_0() {
  builtin.serialise_data(#"") == #"40"
}

test unpend_1() {
  builtin.serialise_data(#"00") == #"4100"
}

test unpend_2() {
  builtin.serialise_data(
    #"4242424242424242424242424242424242424242424242424242424242424242",
  ) == #"58204242424242424242424242424242424242424242424242424242424242424242"
}

test eol_0() {
  builtin.serialise_data(t.End) == #"d87980"
}

test eol_1() {
  builtin.serialise_data(t.Elapse) == #"d87a80"
}

test redeemer_0() {
  builtin.serialise_data(t.Defer) == #"d87980"
}

test redeemer_1() {
  builtin.serialise_data(t.Mutual) == #"d87b80"
}

test redeemer_2() {
  builtin.serialise_data(t.Main([t.StepCont(t.Add), t.StepEol(t.End)])) == #"d87a9f9fd8799fd87980ffd87a9fd87980ffffff"
}

test locked_cheque_encoding_0() {
  builtin.serialise_data(
    t.LockedCheque(
      (
        1,
        2000000,
        60000,
        #"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
      ),
      #"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    ),
  ) == #"d87a9f9f011a001e848019ea605820ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ff"
}

test unlocked_cheque_encoding_0() {
  // EXPECTED TO FAIL: Rust and Aiken use different UnlockedCheque encodings.
  // Rust: constr0([body_with_secret, sig])  vs  Aiken: constr0([body_with_lock, sig, secret])
  builtin.serialise_data(
    t.UnlockedCheque(
      (
        5,
        3000000,
        120000,
        #"e29442e61ad354e5cb0831e2e8359e8fb50cf024ad5a8f407c8f9de63bdf7371",
      ),
      #"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
      #"7777777777777777777777777777777777777777777777777777777777777777",
    ),
  ) == #"d8799f9f051a002dc6c01a0001d4c058207777777777777777777777777777777777777777777777777777777777777777ff584000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ff"
}

test locked_cheque_wellsigned_0() {
  let vk: VerificationKey =
    #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"
  let tag: t.Tag = #"deadbeef"
  let sig =
    #"461030848553591193ba35c9882480e58f1908b22220278b7fa2a2a8942da1dfb2ed044a1e26c74e7b4c6b1a5720fe78d4419e3e64b4e41166e579bfbeaae203"
  wellsigned(
    vk,
    tag,
    (
      0,
      0,
      0,
      #"abababababababababababababababababababababababababababababababab",
    ),
    sig,
  )
}

test locked_cheque_wellsigned_1() {
  let vk: VerificationKey =
    #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"
  let tag: t.Tag = #"deadbeef"
  let sig =
    #"5b2e04f05f305174572ffcbda3e9268d40a42b9505f806b397047fdfd966102ffbebdfff907491f79a34aca75af6a7e343724fe2e10c20daf5adc2d10c29df07"
  wellsigned(
    vk,
    tag,
    (
      1,
      1000000,
      86400000,
      #"abababababababababababababababababababababababababababababababab",
    ),
    sig,
  )
}

test locked_cheque_wellsigned_2() {
  let vk: VerificationKey =
    #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"
  let tag: t.Tag = #"deadbeef"
  let sig =
    #"fb4f8240d34638d5c02310579d03f8bb18405f19c4fd41be2ddb8b70d5bc92a96ded41bde2f47448eee86403706ba322f54e149ae5c77ab79b217e1dadcf3b01"
  wellsigned(
    vk,
    tag,
    (
      42,
      5000000,
      3600000,
      #"abababababababababababababababababababababababababababababababab",
    ),
    sig,
  )
}

test squash_wellsigned_0() {
  let vk: VerificationKey =
    #"8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"
  let tag: t.Tag = #"0102"
  let sig =
    #"ed329b2deaec68ff8def30c01475d2a8e2ba898c3a56f6343d44eee61972f81218688534cc265304b8fed03afda5c093f4284f2058b214457c36d4d2dc152c05"
  wellsigned(vk, tag, (0, 0, []), sig)
}

test squash_wellsigned_1() {
  let vk: VerificationKey =
    #"8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c"
  let tag: t.Tag = #"0102"
  let sig =
    #"c8e4abfcc35b60bc636f46dd97bfba139883c6d79c2d03a4bb72a1d148cff1da2c74e996c5d5ec016d1b7542f751bb41467ada22d411821ae47fa34eef39d20f"
  wellsigned(vk, tag, (1000000, 5, [1, 2, 3]), sig)
}
