import * as codec from "@konduit/codec";
import * as cbor from "@konduit/codec/cbor/codecs/sync";
import { ok } from "neverthrow";
import type { JsonError } from "@konduit/codec/json/codecs";
import { NonNegativeInt } from "@konduit/codec/integers/smallish";
import {
  Lovelace,
  ScriptHash,
  cbor2Ed25519VerificationKeyCodec,
} from "@konduit/konduit-consumer/cardano";
import type { Ed25519VerificationKey } from "@konduit/cardano-keys";
import {
  ChannelTag,
  Index,
  cbor2IndexCodec,
} from "@konduit/konduit-consumer/channel";
import {
  HtlcLock,
  cbor2HtlcLockCodec,
} from "@konduit/konduit-consumer/bitcoin/bolt11";
import { POSIXMilliseconds } from "@konduit/konduit-consumer/time/absolute";
import { Milliseconds } from "@konduit/konduit-consumer/time/duration";
import type { Tagged } from "type-fest";
import {
  asTuple,
  mkConstrTupleCodec,
  mkUint8ArrayCodec,
  plutusListOf,
} from "./cbor.js";

export const cbor2POSIXMillisecondsCodec: cbor.CborCodec<POSIXMilliseconds> =
  codec.pipe(cbor.cbor2IntCodec, POSIXMilliseconds.bigIntCodec);

const nonNegativeInt2ClosePeriod: codec.Codec<
  NonNegativeInt,
  Milliseconds,
  JsonError
> = {
  deserialise: (n) => ok(Milliseconds.fromNonNegativeInt(n)),
  serialise: (duration) => duration as unknown as NonNegativeInt,
};

export const cbor2ClosePeriodCodec: cbor.CborCodec<Milliseconds> = codec.pipe(
  codec.pipe(cbor.cbor2IntCodec, NonNegativeInt.bigIntCodec),
  nonNegativeInt2ClosePeriod,
);

/** Datum::own_hash — a 28-byte script hash. */
export type OwnHash = ScriptHash;

/** konduit_data::Lock — a 32-byte hash. */
export type Lock = HtlcLock;

/** One retired cheque index. */
export type UsedRecord = {
  readonly index: Index;
  readonly amount: Lovelace;
};

export type Used = Tagged<UsedRecord, "Used">;
export namespace Used {
  export const fromRecord = (record: UsedRecord): Used => record as Used;
  export const cborCodec: cbor.CborCodec<Used> = asTuple(
    [cbor2IndexCodec, Lovelace.cborCodec],
    ([index, amount]) => fromRecord({ index, amount }),
    ({ index, amount }) => [index, amount],
  );
}

/** An outstanding adaptor payment. */
export type PendingRecord = {
  readonly amount: Lovelace;
  readonly timeout: POSIXMilliseconds;
  readonly lock: Lock;
};

export type Pending = Tagged<PendingRecord, "Pending">;
export namespace Pending {
  export const fromRecord = (record: PendingRecord): Pending =>
    record as Pending;
  export const cborCodec: cbor.CborCodec<Pending> = asTuple(
    [Lovelace.cborCodec, cbor2POSIXMillisecondsCodec, cbor2HtlcLockCodec],
    ([amount, timeout, lock]) => fromRecord({ amount, timeout, lock }),
    ({ amount, timeout, lock }) => [amount, timeout, lock],
  );
}

export type OpenedStageRecord = {
  readonly subbed: Lovelace;
  readonly useds: Used[];
};

export type OpenedStage = Tagged<
  OpenedStageRecord & { readonly label: "Opened" },
  "OpenedStage"
>;
export namespace OpenedStage {
  export const LABEL = "Opened";
  export const fromRecord = (record: OpenedStageRecord): OpenedStage =>
    ({ ...record, label: LABEL }) as OpenedStage;
  export const cborCodec: cbor.CborCodec<OpenedStage> = mkConstrTupleCodec(
    0,
    [Lovelace.cborCodec, plutusListOf(Used.cborCodec)],
    ([subbed, useds]) => fromRecord({ subbed, useds }),
    ({ subbed, useds }) => [subbed, useds],
  );
}

export type ClosedStageRecord = {
  readonly subbed: Lovelace;
  readonly useds: Used[];
  readonly elapseAt: POSIXMilliseconds;
};

export type ClosedStage = Tagged<
  ClosedStageRecord & { readonly label: "Closed" },
  "ClosedStage"
>;
export namespace ClosedStage {
  export const LABEL = "Closed";
  export const fromRecord = (record: ClosedStageRecord): ClosedStage =>
    ({ ...record, label: LABEL }) as ClosedStage;
  export const cborCodec: cbor.CborCodec<ClosedStage> = mkConstrTupleCodec(
    1,
    [
      Lovelace.cborCodec,
      plutusListOf(Used.cborCodec),
      cbor2POSIXMillisecondsCodec,
    ],
    ([subbed, useds, elapseAt]) => fromRecord({ subbed, useds, elapseAt }),
    ({ subbed, useds, elapseAt }) => [subbed, useds, elapseAt],
  );
}

export type RespondedStageRecord = {
  readonly pendsAmount: Lovelace;
  readonly pendings: Pending[];
};

export type RespondedStage = Tagged<
  RespondedStageRecord & { readonly label: "Responded" },
  "RespondedStage"
>;
export namespace RespondedStage {
  export const LABEL = "Responded";
  export const fromRecord = (record: RespondedStageRecord): RespondedStage =>
    ({ ...record, label: LABEL }) as RespondedStage;
  export const cborCodec: cbor.CborCodec<RespondedStage> = mkConstrTupleCodec(
    2,
    [Lovelace.cborCodec, plutusListOf(Pending.cborCodec)],
    ([pendsAmount, pendings]) => fromRecord({ pendsAmount, pendings }),
    ({ pendsAmount, pendings }) => [pendsAmount, pendings],
  );
}

/**
 * `subbed` is the cumulative amount already subbed out of the channel — *not*
 * the channel's locked lovelace, which lives in the UTxO. `Responded` carries
 * `pendsAmount`, the total of its pendings.
 */
export type Stage = OpenedStage | ClosedStage | RespondedStage;
export namespace Stage {
  export const cborCodec: cbor.CborCodec<Stage> = cbor.altCborCodecs(
    [OpenedStage.cborCodec, ClosedStage.cborCodec, RespondedStage.cborCodec],
    (serOpened, serClosed, serResponded) => (stage) =>
      stage.label === OpenedStage.LABEL
        ? serOpened(stage)
        : stage.label === ClosedStage.LABEL
          ? serClosed(stage)
          : serResponded(stage),
  );

  /** The naked enum, for comparisons and reporting. */
  export type Label = "Opened" | "Closed" | "Responded";
  export const label = (stage: Stage): Label => stage.label;
}

export type ConstantsRecord = {
  readonly tag: ChannelTag;
  readonly addVkey: Ed25519VerificationKey;
  readonly subVkey: Ed25519VerificationKey;
  readonly closePeriod: Milliseconds;
};

export type Constants = Tagged<ConstantsRecord, "Constants">;
export namespace Constants {
  export const fromRecord = (record: ConstantsRecord): Constants =>
    record as Constants;
  export const cborCodec: cbor.CborCodec<Constants> = asTuple(
    [
      ChannelTag.cborCodec,
      cbor2Ed25519VerificationKeyCodec,
      cbor2Ed25519VerificationKeyCodec,
      cbor2ClosePeriodCodec,
    ],
    ([tag, addVkey, subVkey, closePeriod]) =>
      fromRecord({ tag, addVkey, subVkey, closePeriod }),
    ({ tag, addVkey, subVkey, closePeriod }) => [
      tag,
      addVkey,
      subVkey,
      closePeriod,
    ],
  );
}

export type DatumRecord = {
  readonly ownHash: OwnHash;
  readonly constants: Constants;
  readonly stage: Stage;
};

export type Datum = Tagged<DatumRecord, "Datum">;
export namespace Datum {
  export const fromRecord = (record: DatumRecord): Datum => record as Datum;
  export const cborCodec: cbor.CborCodec<Datum> = asTuple(
    [ScriptHash.cborCodec, Constants.cborCodec, Stage.cborCodec],
    ([ownHash, constants, stage]) => fromRecord({ ownHash, constants, stage }),
    ({ ownHash, constants, stage }) => [ownHash, constants, stage],
  );

  /** The raw bytes the indexer stores — `cborCodec` speaks the CBOR AST. */
  export const uint8ArrayCodec: codec.Codec<Uint8Array, Datum, JsonError> =
    mkUint8ArrayCodec(cborCodec);
}
