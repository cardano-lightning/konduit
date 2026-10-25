import { describe, expect, it } from "vitest";
import { unwrapOrPanic } from "@konduit/konduit-consumer/neverthrow";
import { Datum } from "../src/konduit/datum.js";
import { Redeemer, Step, Unpend } from "../src/konduit/redeemer.js";
import * as hexString from "@konduit/codec/hexString";
import * as uint8Array from "@konduit/codec/uint8Array";
import {
  CLOSE_ELAPSE_AT_MS,
  DATUM_ADDED,
  DATUM_CLOSED,
  DATUM_OPENED,
  DATUM_RESPONDED,
  DATUM_SUBBED,
  FIXTURE_ADD_VKEY_HEX,
  FIXTURE_CLOSE_PERIOD_MS,
  FIXTURE_OWN_HASH_HEX,
  FIXTURE_SUB_VKEY_HEX,
  FIXTURE_TAG_HEX,
  OPENED_LOVELACE,
  REDEEMER_ADD,
  REDEEMER_ADD_THEN_CLOSE,
  REDEEMER_CLOSE,
  REDEEMER_DEFER,
  REDEEMER_ELAPSE,
  REDEEMER_END,
  REDEEMER_EXPIRE,
  REDEEMER_MUTUAL,
  REDEEMER_RESPOND,
  REDEEMER_SUB,
  REDEEMER_UNLOCK,
  SUBBED_TOTAL,
  SUBBED_USEDS,
} from "./support/konduitCbor.js";

/**
 * hex → bytes → the decoded value. The library exposes the *codec*, not a
 * decode helper, so the hex step lives here.
 */
const decodeDatumHex = (hex: string) =>
  uint8Array.stringCodec
    .deserialise(hex)
    .andThen(Datum.uint8ArrayCodec.deserialise);

const decodeRedeemerHex = (hex: string) =>
  uint8Array.stringCodec
    .deserialise(hex)
    .andThen(Redeemer.uint8ArrayCodec.deserialise);

/** Unwrap a decode, failing the test with the codec's own message. */
const datumOf = (hex: string): Datum =>
  unwrapOrPanic(decodeDatumHex(hex), "datum decode failed");

const redeemerOf = (hex: string): Redeemer =>
  unwrapOrPanic(decodeRedeemerHex(hex), "redeemer decode failed");

/** The steps of a `Main` redeemer, or `[]` for the other variants. */
const stepsOf = (redeemer: Redeemer) =>
  redeemer.kind === "Main" ? redeemer.steps : [];

describe("hex helpers", () => {
  it("round-trips bare hex — the konduit convention, no 0x prefix", () => {
    expect(
      hexString.fromUint8Array(
        uint8Array.stringCodec.deserialise("0a0bff")._unsafeUnwrap(),
      ),
    ).toBe("0a0bff");
    expect(
      hexString.fromUint8Array(
        uint8Array.stringCodec.deserialise("")._unsafeUnwrap(),
      ),
    ).toBe("");
  });

  it("rejects malformed hex instead of guessing", () => {
    expect(uint8Array.stringCodec.deserialise("zz").isErr()).toBe(true); // not hex
    expect(uint8Array.stringCodec.deserialise("abc").isErr()).toBe(true); // odd length
    expect(uint8Array.stringCodec.deserialise("0x0102").isErr()).toBe(true); // konduit hex has no prefix
  });
});

describe("datum decoding", () => {
  it("decodes constants identically across every stage variant", () => {
    for (const hex of [
      DATUM_OPENED,
      DATUM_ADDED,
      DATUM_SUBBED,
      DATUM_CLOSED,
      DATUM_RESPONDED,
    ]) {
      const { ownHash, constants } = datumOf(hex);
      expect(hexString.fromUint8Array(ownHash)).toBe(FIXTURE_OWN_HASH_HEX);
      expect(hexString.fromUint8Array(constants.tag)).toBe(FIXTURE_TAG_HEX);
      expect(hexString.fromUint8Array(constants.addVkey.key)).toBe(
        FIXTURE_ADD_VKEY_HEX,
      );
      expect(hexString.fromUint8Array(constants.subVkey.key)).toBe(
        FIXTURE_SUB_VKEY_HEX,
      );
      expect(constants.closePeriod).toBe(FIXTURE_CLOSE_PERIOD_MS);
    }
  });

  it("decodes a freshly opened channel as subbed = 0", () => {
    expect(datumOf(DATUM_OPENED).stage).toEqual({
      label: "Opened",
      subbed: 0n,
      useds: [],
    });
  });

  it("carries no channel amount in the datum", () => {
    // The trap this whole module exists to avoid: `Stage::Opened`'s first field
    // is the running `subbed` total, *not* the locked amount. The `minicbor`
    // impl in konduit-data destructures it as `Stage::Opened(amount, useds)`,
    // which reads as though it were the amount. `konduit_tx::open` settles it —
    // a fresh channel encodes 0.
    const stage = datumOf(DATUM_OPENED).stage;
    expect(stage).not.toHaveProperty("amount");
    expect(Object.keys(stage).sort()).toEqual(["label", "subbed", "useds"]);
    expect(stage.label).toBe("Opened");
    if (stage.label !== "Opened") return;
    expect(stage.subbed).toBe(0n);
    expect(stage.subbed).not.toBe(OPENED_LOVELACE);
  });

  it("leaves the datum byte-identical across an add", () => {
    // `add.ak` requires `stage_in == stage_out`; only the UTxO value moves, and
    // that lives in the store's `lovelace` column, not in the datum.
    expect(DATUM_ADDED).toBe(DATUM_OPENED);
    expect(datumOf(DATUM_ADDED)).toEqual(datumOf(DATUM_OPENED));
  });

  it("decodes the subbed total and the retired cheque indexes", () => {
    const { stage } = datumOf(DATUM_SUBBED);
    expect(stage.label).toBe("Opened");
    if (stage.label !== "Opened") return;
    expect(stage.subbed).toBe(SUBBED_TOTAL);
    expect(stage.useds).toEqual(SUBBED_USEDS);
  });

  it("carries the subbed total across a close, plus its elapse_at", () => {
    expect(datumOf(DATUM_CLOSED).stage).toEqual({
      label: "Closed",
      subbed: SUBBED_TOTAL,
      useds: [],
      elapseAt: CLOSE_ELAPSE_AT_MS,
    });
  });

  it("decodes a responded channel, whose leading field is the pendings total", () => {
    const { stage } = datumOf(DATUM_RESPONDED);
    expect(stage.label).toBe("Responded");
    if (stage.label !== "Responded") return;
    expect(stage.pendsAmount).toBe(3_000_000n);
    expect(stage.pendings).toEqual([
      {
        amount: 3_000_000n,
        timeout: CLOSE_ELAPSE_AT_MS,
        lock: uint8Array.stringCodec
          .deserialise("07".repeat(32))
          ._unsafeUnwrap(),
      },
    ]);
  });

  it("keeps the constants identical while the stage evolves", () => {
    const reference = datumOf(DATUM_OPENED).constants;
    for (const hex of [DATUM_SUBBED, DATUM_CLOSED, DATUM_RESPONDED]) {
      expect(datumOf(hex).constants).toEqual(reference);
    }
  });

  it("round-trips every fixture through encode → decode", () => {
    for (const hex of [
      DATUM_OPENED,
      DATUM_SUBBED,
      DATUM_CLOSED,
      DATUM_RESPONDED,
    ]) {
      const datum = datumOf(hex);
      expect(
        decodeDatumHex(
          hexString.fromUint8Array(Datum.uint8ArrayCodec.serialise(datum)),
        ),
      ).toEqual(decodeDatumHex(hex));
    }
  });

  it("re-encodes the Rust datum fixture byte-for-byte", () => {
    expect(
      hexString.fromUint8Array(
        Datum.uint8ArrayCodec.serialise(datumOf(DATUM_OPENED)),
      ),
    ).toBe(DATUM_OPENED);
  });

  it("rejects a truncated datum rather than guessing", () => {
    expect(decodeDatumHex(DATUM_OPENED.slice(0, 40)).isErr()).toBe(true);
  });

  it("rejects a redeemer blob fed to the datum codec and vice versa", () => {
    expect(decodeDatumHex(REDEEMER_ADD).isErr()).toBe(true);
    expect(decodeRedeemerHex(DATUM_OPENED).isErr()).toBe(true);
  });
});

describe("redeemer decoding", () => {
  it("recognises the consumer-side continuations", () => {
    expect(Redeemer.labels(redeemerOf(REDEEMER_ADD))).toEqual(["Cont:Add"]);
    expect(Redeemer.labels(redeemerOf(REDEEMER_CLOSE))).toEqual(["Cont:Close"]);
  });

  it("recognises the end-of-life continuations", () => {
    expect(Redeemer.labels(redeemerOf(REDEEMER_END))).toEqual(["Eol:End"]);
    expect(Redeemer.labels(redeemerOf(REDEEMER_ELAPSE))).toEqual([
      "Eol:Elapse",
    ]);
  });

  it("recognises the non-Main redeemers", () => {
    expect(Redeemer.labels(redeemerOf(REDEEMER_MUTUAL))).toEqual(["Mutual"]);
    expect(Redeemer.labels(redeemerOf(REDEEMER_DEFER))).toEqual(["Defer"]);
  });

  it("decodes a batch redeemer as more than one step", () => {
    expect(Redeemer.labels(redeemerOf(REDEEMER_ADD_THEN_CLOSE))).toEqual([
      "Cont:Add",
      "Cont:Close",
    ]);
  });

  it("summarises an adaptor sub: squash amount/index and unlocked count", () => {
    const redeemer = redeemerOf(REDEEMER_SUB);
    expect(Redeemer.labels(redeemer)).toEqual(["Cont:Sub"]);
    const step = stepsOf(redeemer)[0]!;
    expect(Step.label(step)).toBe("Cont:Sub");
    if (!("cont" in step) || step.cont.kind !== "Sub") {
      throw new Error("expected a Sub step");
    }
    expect(step.cont.squash.amount).toBe(2_000_000n);
    expect(step.cont.squash.index).toBe(0n);
    expect(step.cont.items).toBe(1);
  });

  it("summarises an adaptor receipt (respond) with its cheque count", () => {
    const step = stepsOf(redeemerOf(REDEEMER_RESPOND))[0]!;
    if (!("cont" in step) || step.cont.kind !== "Respond") {
      throw new Error("expected a Respond step");
    }
    expect(step.cont.items).toBe(1);
    expect(step.cont.squash.amount).toBe(2_000_000n);
  });

  it("decodes the unlock/expire unpends", () => {
    const unlock = stepsOf(redeemerOf(REDEEMER_UNLOCK))[0]!;
    if (!("cont" in unlock) || unlock.cont.kind !== "Unlock") {
      throw new Error("expected an Unlock step");
    }
    // `Unpend` is a bytestring: 32 bytes = Unlock.
    expect(unlock.cont.unpends.map(Unpend.kindOf)).toEqual(["Unlock"]);

    const expire = stepsOf(redeemerOf(REDEEMER_EXPIRE))[0]!;
    if (!("cont" in expire) || expire.cont.kind !== "Expire") {
      throw new Error("expected an Expire step");
    }
    // 1 byte = Expire — *not* a constructor.
    expect(expire.cont.unpends.map(Unpend.kindOf)).toEqual(["Expire"]);
  });

  it("round-trips every fixture through encode → decode", () => {
    for (const hex of [
      REDEEMER_ADD,
      REDEEMER_ADD_THEN_CLOSE,
      REDEEMER_MUTUAL,
      REDEEMER_DEFER,
      REDEEMER_END,
      REDEEMER_SUB,
      REDEEMER_RESPOND,
      REDEEMER_UNLOCK,
      REDEEMER_EXPIRE,
    ]) {
      const redeemer = redeemerOf(hex);
      expect(
        Redeemer.labels(
          decodeRedeemerHex(
            hexString.fromUint8Array(
              Redeemer.uint8ArrayCodec.serialise(redeemer),
            ),
          )._unsafeUnwrap(),
        ),
      ).toEqual(Redeemer.labels(redeemer));
    }
  });

  it("re-encodes a unit redeemer byte-for-byte", () => {
    expect(
      hexString.fromUint8Array(
        Redeemer.uint8ArrayCodec.serialise(redeemerOf(REDEEMER_ADD)),
      ),
    ).toBe(REDEEMER_ADD);
  });

  it("rejects a non-redeemer blob", () => {
    expect(decodeRedeemerHex("ffff").isErr()).toBe(true);
  });
});

describe("namespace codecs", () => {
  it("are exposed on the type, not as loose module bindings", () => {
    // The konduit-consumer convention: `Datum.cborCodec`, `Used.cborCodec`, …
    expect(typeof Datum.cborCodec.deserialise).toBe("function");
    expect(typeof Datum.cborCodec.serialise).toBe("function");
    expect(typeof Redeemer.labels).toBe("function");
    expect(typeof Step.label).toBe("function");
  });
});
