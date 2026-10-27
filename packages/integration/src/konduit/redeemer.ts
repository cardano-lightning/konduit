import * as codec from "@konduit/codec";
import * as cbor from "@konduit/codec/cbor/codecs/sync";
import type { JsonError } from "@konduit/codec/json/codecs";
import { type Cbor, cborNull } from "@konduit/codec/cbor/core";
import { mkOrdForUint8Array } from "@konduit/codec/tagged";
import type { Tagged } from "type-fest";
import { Lovelace } from "@konduit/konduit-consumer/cardano";
import { Index, cbor2IndexCodec } from "@konduit/konduit-consumer/channel";
import {
  anyCbor,
  asTuple,
  mkConstrTupleCodec,
  mkUnitConstrCodec,
  mkUint8ArrayCodec,
  plutusListOf,
} from "./cbor.js";

/**
 * @file The `redeemer` blob the `konduit-indexer` store keeps — the proof
 * that a channel transition was authorised.
 *
 * Conventions and wire layout: ../../README.md.
 */

/** `Unpend`: empty = Continue, 32 bytes = Unlock, anything else = Expire. */
export type Unpend = Tagged<Uint8Array, "Unpend">;
export namespace Unpend {
  export type Kind = "Continue" | "Expire" | "Unlock";
  export const CONTINUE = new Uint8Array(0) as Unpend;
  export const EXPIRE = new Uint8Array([0]) as Unpend;
  export const UNLOCK = new Uint8Array(32) as Unpend;
  export const kindOf = (unpend: Unpend): Kind => {
    if (unpend.length === 0) return "Continue";
    return unpend.length === 32 ? "Unlock" : "Expire";
  };

  export const fromKind = (kind: Kind): Unpend =>
    kind === "Continue" ? CONTINUE : kind === "Unlock" ? UNLOCK : EXPIRE;
  export const cborCodec: cbor.CborCodec<Unpend> = codec.rmap(
    cbor.cbor2ByteStringCodec,
    (bytes) => bytes as Unpend,
    (unpend) => unpend,
  );
  export const ord = mkOrdForUint8Array<Unpend>();
}

/** `SquashBody` is `[owed, index, exclude]`. */
export type SquashBodyRecord = {
  readonly amount: Lovelace;
  readonly index: Index;
  readonly exclude: Index[];
};

export type SquashBody = Tagged<SquashBodyRecord, "SquashBody">;
export namespace SquashBody {
  export const fromRecord = (record: SquashBodyRecord): SquashBody =>
    record as SquashBody;
  export const cborCodec: cbor.CborCodec<SquashBody> = asTuple(
    [Lovelace.cborCodec, cbor2IndexCodec, plutusListOf(cbor2IndexCodec)],
    ([amount, index, exclude]) => fromRecord({ amount, index, exclude }),
    ({ amount, index, exclude }) => [amount, index, exclude],
  );
}

/** `[body, signature]`; only the body is kept — the adaptor verifies it. The
 * inverse mapping hands `SquashBody.cborCodec` the domain object, the one
 * nested codec with a non-identity serialiser. */
export type Squash = Tagged<SquashBody, "Squash">;
export namespace Squash {
  export const fromBody = (body: SquashBody): Squash => body as Squash;
  export const bodyOf = (squash: Squash): SquashBody => squash;
  export const cborCodec: cbor.CborCodec<Squash> = asTuple(
    [SquashBody.cborCodec, anyCbor],
    ([body]) => fromBody(body),
    (squash) => [bodyOf(squash), cborNull],
  );
}

export type ContRecord = {
  readonly squash: Squash;
  readonly items: number;
};

export type AddCont = Tagged<{ readonly kind: "Add" }, "AddCont">;
export type SubCont = Tagged<ContRecord & { readonly kind: "Sub" }, "SubCont">;
export type CloseCont = Tagged<{ readonly kind: "Close" }, "CloseCont">;
export type RespondCont = Tagged<
  ContRecord & { readonly kind: "Respond" },
  "RespondCont"
>;
export type UnlockCont = Tagged<
  { readonly kind: "Unlock"; readonly unpends: Unpend[] },
  "UnlockCont"
>;
export type ExpireCont = Tagged<
  { readonly kind: "Expire"; readonly unpends: Unpend[] },
  "ExpireCont"
>;
export type Cont =
  | AddCont
  | SubCont
  | CloseCont
  | RespondCont
  | UnlockCont
  | ExpireCont;
export namespace Cont {
  export type Kind =
    | "Add"
    | "Sub"
    | "Close"
    | "Respond"
    | "Unlock"
    | "Expire"
    /** The naked enum, so `switch (cont.kind)` is exhaustiveness-checked. */
    | "Unknown";

  /** `Sub` (1) and `Respond` (3): a squash plus a list we only count. */
  const mkSquashContCodec = <C extends SubCont | RespondCont>(
    index: number,
    kind: C["kind"],
  ): cbor.CborCodec<Cont> =>
    mkConstrTupleCodec(
      index,
      [Squash.cborCodec, plutusListOf(anyCbor)],
      ([squash, items]) =>
        ({ kind, squash, items: items.length }) as unknown as Cont,
      (cont) =>
        cont.kind === "Sub" || cont.kind === "Respond"
          ? [cont.squash, []]
          : [
              Squash.fromBody(
                SquashBody.fromRecord({
                  amount: 0n as Lovelace,
                  index: Index.zero,
                  exclude: [],
                }),
              ),
              [],
            ],
    );

  /** `Unlock` (4) and `Expire` (5). */
  const mkUnpendContCodec = (
    index: number,
    kind: "Unlock" | "Expire",
  ): cbor.CborCodec<Cont> =>
    mkConstrTupleCodec(
      index,
      [plutusListOf(Unpend.cborCodec)],
      ([unpends]) => ({ kind, unpends }) as Cont,
      (cont) => [
        cont.kind === "Unlock" || cont.kind === "Expire" ? cont.unpends : [],
      ],
    );
  export const addCodec = mkUnitConstrCodec(
    0,
    (): AddCont => ({ kind: "Add" }) as AddCont,
  );
  export const subCodec = mkSquashContCodec(1, "Sub");
  export const closeCodec = mkUnitConstrCodec(
    2,
    (): CloseCont => ({ kind: "Close" }) as CloseCont,
  );
  export const respondCodec = mkSquashContCodec(3, "Respond");
  export const unlockCodec = mkUnpendContCodec(4, "Unlock");
  export const expireCodec = mkUnpendContCodec(5, "Expire");
  export const cborCodec: cbor.CborCodec<Cont> = cbor.altCborCodecs(
    [addCodec, subCodec, closeCodec, respondCodec, unlockCodec, expireCodec],
    (serAdd, serSub, serClose, serRespond, serUnlock, serExpire) =>
      (cont: Cont): Cbor => {
        switch (cont.kind) {
          case "Add":
            return serAdd(cont);
          case "Sub":
            return serSub(cont);
          case "Close":
            return serClose(cont);
          case "Respond":
            return serRespond(cont);
          case "Unlock":
            return serUnlock(cont);
          case "Expire":
            return serExpire(cont);
        }
      },
  );

  /** One-line projection: `Cont:Sub`, `Cont:Add`, … */
  export const label = (cont: Cont): string => `Cont:${cont.kind}`;
}

export type EolRecord = { readonly kind: "End" | "Elapse" };
export type Eol = Tagged<EolRecord, "Eol">;
export namespace Eol {
  export const END = { kind: "End" } as Eol;
  export const ELAPSE = { kind: "Elapse" } as Eol;
  export const cborCodec: cbor.CborCodec<Eol> = cbor.altCborCodecs(
    [mkUnitConstrCodec(0, () => END), mkUnitConstrCodec(1, () => ELAPSE)],
    (serEnd, serElapse) => (eol: Eol) =>
      eol.kind === "End" ? serEnd(END) : serElapse(ELAPSE),
  );
  export const label = (eol: Eol): string => `Eol:${eol.kind}`;
}

export type ContStep = Tagged<{ readonly cont: Cont }, "ContStep">;
export type EolStep = Tagged<{ readonly eol: Eol }, "EolStep">;
export type Step = ContStep | EolStep;
export namespace Step {
  export const contCodec: cbor.CborCodec<ContStep> = mkConstrTupleCodec(
    0,
    [Cont.cborCodec],
    ([cont]) => ({ cont }) as ContStep,
    (step) => [
      (step as ContStep).cont ??
        Cont.addCodec.serialise({ kind: "Add" } as AddCont),
    ],
  );
  export const eolCodec: cbor.CborCodec<EolStep> = mkConstrTupleCodec(
    1,
    [Eol.cborCodec],
    ([eol]) => ({ eol }) as EolStep,
    (step) => [(step as EolStep).eol ?? Eol.END],
  );
  export const cborCodec: cbor.CborCodec<Step> = cbor.altCborCodecs(
    [contCodec, eolCodec],
    (serCont, serEol) =>
      (step: Step): Cbor =>
        "cont" in step ? serCont(step) : serEol(step),
  );
  export const label = (step: Step): string =>
    "cont" in step ? Cont.label(step.cont) : Eol.label(step.eol);
}

export type MainRedeemer = Tagged<
  { readonly kind: "Main"; readonly steps: Step[] },
  "MainRedeemer"
>;
export type Redeemer =
  | Tagged<{ readonly kind: "Defer" }, "DeferRedeemer">
  | MainRedeemer
  | Tagged<{ readonly kind: "Mutual" }, "MutualRedeemer">;
export namespace Redeemer {
  export const deferCodec = mkUnitConstrCodec(
    0,
    (): Redeemer => ({ kind: "Defer" }) as Redeemer,
  );
  export const mainCodec: cbor.CborCodec<MainRedeemer> = mkConstrTupleCodec(
    1,
    [plutusListOf(Step.cborCodec)],
    ([steps]) => ({ kind: "Main", steps }) as MainRedeemer,
    (redeemer) => [(redeemer as MainRedeemer).steps],
  );
  export const mutualCodec = mkUnitConstrCodec(
    2,
    (): Redeemer => ({ kind: "Mutual" }) as Redeemer,
  );
  export const cborCodec: cbor.CborCodec<Redeemer> = cbor.altCborCodecs(
    [deferCodec, mainCodec, mutualCodec],
    (serDefer, serMain, serMutual) =>
      (redeemer: Redeemer): Cbor => {
        switch (redeemer.kind) {
          case "Defer":
            return serDefer(redeemer);
          case "Main":
            return serMain(redeemer);
          case "Mutual":
            return serMutual(redeemer);
        }
      },
  );

  /** The redeemer's step kinds: `["Cont:Add"]`, `["Eol:End"]`, … */
  export const labels = (redeemer: Redeemer): string[] =>
    redeemer.kind === "Main" ? redeemer.steps.map(Step.label) : [redeemer.kind];

  /** The raw bytes the indexer stores — `cborCodec` speaks the CBOR AST. */
  export const uint8ArrayCodec: codec.Codec<Uint8Array, Redeemer, JsonError> =
    mkUint8ArrayCodec(cborCodec);
}
