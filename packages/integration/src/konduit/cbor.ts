import * as codec from "@konduit/codec";
import * as cbor from "@konduit/codec/cbor/codecs/sync";
import {
  type Cbor,
  briefCbor,
  type CborTag,
  isCborTaggedValue,
} from "@konduit/codec/cbor/core";
import type { JsonError } from "@konduit/codec/json/codecs";
import { err } from "neverthrow";

/**
 * @file The Plutus/CBOR primitives the konduit datum and redeemer codecs are
 * built from.
 *
 * Nothing in here is konduit specific — these are the encoding rules Plutus
 * forces on us: a constructor is `tag(121 + n)` around a field list, and
 * `plutus_list` mixes definite and indefinite arrays.
 */

/** The four leaves Plutus encoding needs. */
export const CONSTR_TAG_BASE = 121;

/** Pass any node through — for lists where only the item count matters. */
export const anyCbor = codec.mkIdentityCodec<Cbor, JsonError>();

/** The shape `tupleOf` produces for a given list of field codecs. */
export type TupleOut<Cs extends readonly cbor.CborCodec<any>[]> = [
  ...{
    [K in keyof Cs]: Cs[K] extends cbor.CborCodec<infer O> ? O : never;
  },
];

/** Plutus lists are definite when empty and indefinite otherwise. */
export const plutusListOf = <T>(item: cbor.CborCodec<T>): cbor.CborCodec<T[]> =>
  cbor.altCborCodecs(
    [
      cbor.arrayOf(cbor.definiteLength, item),
      cbor.arrayOf(cbor.indefiniteLength, item),
    ],
    (serialiseDefinite, serialiseIndefinite) => (values: T[]) =>
      values.length === 0
        ? serialiseDefinite(values)
        : serialiseIndefinite(values),
  );

/** A Plutus constructor: `tag(121 + index)` around a field list. */
export const constrCodec = <T>(
  index: number,
  fields: cbor.CborCodec<T>,
): cbor.CborCodec<T> => ({
  deserialise: (data) => {
    if (!isCborTaggedValue(data)) {
      return err(`expected constructor ${index}, got ${briefCbor(data)}`);
    }
    if (data.tag !== CONSTR_TAG_BASE + index) {
      return err(
        `expected constructor ${index}, got constructor ${data.tag - CONSTR_TAG_BASE}`,
      );
    }
    return fields.deserialise(data.value);
  },
  serialise: (value) => ({
    tag: (CONSTR_TAG_BASE + index) as CborTag,
    value: fields.serialise(value),
  }),
});

/** `rmap` sugar for a fixed-arity tuple. */
export const asTuple = <Cs extends readonly cbor.CborCodec<any>[], O>(
  cs: [...Cs],
  to: (parts: TupleOut<Cs>) => O,
  from: (value: O) => TupleOut<Cs>,
): cbor.CborCodec<O> =>
  codec.rmap(cbor.tupleOf(cbor.indefiniteLength, ...cs), to, from);

/** One enum variant: constructor `index` around a field tuple. */
export const mkConstrTupleCodec = <
  Cs extends readonly cbor.CborCodec<any>[],
  O,
>(
  index: number,
  cs: [...Cs],
  to: (parts: TupleOut<Cs>) => O,
  from: (value: O) => TupleOut<Cs>,
): cbor.CborCodec<O> =>
  codec.rmap(
    constrCodec(index, cbor.tupleOf(cbor.indefiniteLength, ...cs)),
    to,
    from,
  );

/** A constructor with no fields — `Add`, `Close`, `Defer`, … */
export const mkUnitConstrCodec = <T>(
  index: number,
  make: () => T,
): cbor.CborCodec<T> =>
  codec.rmap(
    constrCodec(index, cbor.tupleOf(cbor.definiteLength)),
    make,
    (): [] => [],
  );

/** `Uint8Array` ↦ decoded value — the only place the AST is walked. */
export const mkUint8ArrayCodec = <O>(
  ast: cbor.CborCodec<O>,
): codec.Codec<Uint8Array, O, JsonError> => ({
  deserialise: (bytes) => cbor.deserialiseCbor(bytes).andThen(ast.deserialise),
  serialise: (value) => cbor.serialiseCbor(ast.serialise(value)),
});
