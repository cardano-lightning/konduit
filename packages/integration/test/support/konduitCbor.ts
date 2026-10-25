/**
 * @file Byte-exact `konduit_data` CBOR fixtures.
 *
 * Every hex string below was produced by the **real** Rust encoder, not by a
 * TypeScript re-implementation — that is the whole point. If we encoded the
 * fixtures with our own decoder's assumptions, the round-trip tests would only
 * prove that the decoder agrees with itself.
 *
 * To regenerate (the generator is throw-away, so it is not kept in the repo):
 *
 * ```rust
 * // packages/konduit/data/src/lib.rs  ->  #[cfg(test)] mod fixture_dump;
 * // packages/konduit/data/src/fixture_dump.rs
 * fn dump<T: minicbor::Encode<()>>(label: &str, v: &T) {
 *     println!("{label} = {}", hex::encode(minicbor::to_vec(v).unwrap()));
 * }
 * ```
 *
 * then `cargo test -p konduit-data --features json -- --nocapture fixture_dump`.
 *
 * The values used throughout:
 *
 * ```
 * own_hash   = 0x00..01                      (28 bytes)
 * tag        = 0102030405060708               (8 bytes)
 * add_vkey   = 0x11..11                       (32 bytes)
 * sub_vkey   = 0x22..22                       (32 bytes)
 * close      = 86_400_000 ms  (24 h)
 * ```
 */

/** `Datum::constants.close_period` shared by every fixture datum. */
export const FIXTURE_CLOSE_PERIOD_MS = 86_400_000;
export const FIXTURE_TAG_HEX = "0102030405060708";
export const FIXTURE_ADD_VKEY_HEX =
  "1111111111111111111111111111111111111111111111111111111111111111";
export const FIXTURE_SUB_VKEY_HEX =
  "2222222222222222222222222222222222222222222222222222222222222222";
export const FIXTURE_OWN_HASH_HEX = `${"00".repeat(27)}01`;

/**
 * `Stage::Opened(subbed = 0, useds = [])` — a freshly opened channel.
 *
 * Note the **zero**: `konduit_tx::open` builds
 * `Variables::new(amount, Stage::Opened(0, vec![]))`, so the datum never
 * carries the channel's lovelace. That lives in the UTxO value, which the store
 * keeps in the `channel.lovelace` / `step.lovelace` column.
 */
export const DATUM_OPENED =
  "9f581c000000000000000000000000000000000000000000000000000000019f48010203040506070858201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221a05265c00ffd8799f0080ffff";

/**
 * `Stage::Opened(subbed = 0, useds = [])` — after an `add`.
 *
 * Byte-for-byte identical to {@link DATUM_OPENED}, on purpose: `add.ak` requires
 * `stage_in == stage_out`, so an add moves value without touching the stage.
 * Only the `lovelace` column changes.
 */
export const DATUM_ADDED =
  "9f581c000000000000000000000000000000000000000000000000000000019f48010203040506070858201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221a05265c00ffd8799f0080ffff";

/**
 * `Stage::Opened(subbed = 7_000_000, [Used{1, 3M}, Used{2, 4M}])` — after a
 * `sub` retiring cheque indexes 1 and 2.
 */
export const DATUM_SUBBED =
  "9f581c000000000000000000000000000000000000000000000000000000019f48010203040506070858201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221a05265c00ffd8799f1a006acfc09f9f011a002dc6c0ff9f021a003d0900ffffffff";

/** `Stage::Closed(subbed = 7_000_000, [], elapse_at = 1_700_000_000_000 ms)`. */
export const DATUM_CLOSED =
  "9f581c000000000000000000000000000000000000000000000000000000019f48010203040506070858201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221a05265c00ffd87a9f1a006acfc0801b0000018bcfe56800ffff";

/** `Stage::Responded(pendings_amount = 3_000_000, [Pending{3_000_000, …, 0x07..07}])`. */
export const DATUM_RESPONDED =
  "9f581c000000000000000000000000000000000000000000000000000000019f48010203040506070858201111111111111111111111111111111111111111111111111111111111111111582022222222222222222222222222222222222222222222222222222222222222221a05265c00ffd87b9f1a002dc6c09f9f1a002dc6c01b0000018bcfe5680058200707070707070707070707070707070707070707070707070707070707070707ffffffff";

/** `Redeemer::Main([Step::Cont(Cont::Add)])`. */
export const REDEEMER_ADD = "d87a9f9fd8799fd87980ffffff";
/** `Redeemer::Main([Step::Cont(Cont::Close)])`. */
export const REDEEMER_CLOSE = "d87a9f9fd8799fd87b80ffffff";
/** `Redeemer::Main([Step::Cont(Cont::Add), Step::Cont(Cont::Close)])`. */
export const REDEEMER_ADD_THEN_CLOSE =
  "d87a9f9fd8799fd87980ffd8799fd87b80ffffff";
/** `Redeemer::Mutual` — what a bare `close` spends with. */
export const REDEEMER_MUTUAL = "d87b80";
/** `Redeemer::Defer`. */
export const REDEEMER_DEFER = "d87980";
/** `Redeemer::Main([Step::Eol(Eol::End)])`. */
export const REDEEMER_END = "d87a9f9fd87a9fd87980ffffff";
/** `Redeemer::Main([Step::Eol(Eol::Elapse)])`. */
export const REDEEMER_ELAPSE = "d87a9f9fd87a9fd87a80ffffff";
/** `Redeemer::Main([Step::Cont(Cont::Sub(squash, [unlocked]))])`. */
export const REDEEMER_SUB =
  "d87a9f9fd8799fd87a9f9f9f1a001e84800080ff58402e45783fe4363653aac86b6c1f80a6beba6a6d8e0977654f3e28da1855fde8e6c93125547cb3b48f761b38f6d018b7209b1edd199678760ad7730fbf32f97100ff9f9f9f011a002dc6c01b0000018bcfe5680058200000000000000000000000000000000000000000000000000000000000000000ff58403882b2953fa7386c65f8258906fa839507aa91c06095eec1cd02e3fa75023c02f4520e44cd4898898469e032726ebe59b8ae890d50b062f8c255fa84c44d1b01ffffffffffff";
/** `Redeemer::Main([Step::Cont(Cont::Respond(squash, [cheque]))])`. */
export const REDEEMER_RESPOND =
  "d87a9f9fd8799fd87c9f9f9f1a001e84800080ff58402e45783fe4363653aac86b6c1f80a6beba6a6d8e0977654f3e28da1855fde8e6c93125547cb3b48f761b38f6d018b7209b1edd199678760ad7730fbf32f97100ff9fd87a9f9f021a003d09001b0000018bcfe5680058200909090909090909090909090909090909090909090909090909090909090909ff584010d470812e38459d01632f07701362bd167caa0164fd0eddae5a5efbcf9905dd716709daec376d3fc7ea7dab333e0f1ef97b76616be946d15e87b659ed8fe008ffffffffffff";
/** `Redeemer::Main([Step::Cont(Cont::Unlock([Unpend::Unlock(0x03..03)]))])`. */
export const REDEEMER_UNLOCK =
  "d87a9f9fd8799fd87d9f9f58200303030303030303030303030303030303030303030303030303030303030303ffffffffff";
/** `Redeemer::Main([Step::Cont(Cont::Expire([Unpend::Expire]))])`. */
export const REDEEMER_EXPIRE = "d87a9f9fd8799fd87e9f9f4100ffffffffff";

/**
 * The UTxO values the fixture thread below pairs with the datums above.
 *
 * The datum does **not** carry these — they are the `lovelace` column of the
 * `channel` / `step` rows, which is the only place the channel's value lives.
 */
export const OPENED_LOVELACE = 20_000_000n;
export const ADDED_LOVELACE = 100_000_000n;
export const SUBBED_LOVELACE = 91_000_000n;
/** `subbed` recorded in {@link DATUM_SUBBED} and {@link DATUM_CLOSED}. */
export const SUBBED_TOTAL = 7_000_000n;
/** `elapse_at` carried by {@link DATUM_CLOSED}. */
export const CLOSE_ELAPSE_AT_MS = 1_700_000_000_000;
/** Index/amount pairs retired by {@link DATUM_SUBBED}. */
export const SUBBED_USEDS = [
  { index: 1n, amount: 3_000_000n },
  { index: 2n, amount: 4_000_000n },
];
