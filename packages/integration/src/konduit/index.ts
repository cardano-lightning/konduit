/**
 * @file CBOR codecs for the `datum` and `redeemer` blobs the `konduit-indexer`
 * store keeps. Conventions and wire layout: ../../README.md.
 *
 * These are konduit protocol codecs, not indexer internals — hence the
 * `konduit/` directory. The split is:
 *
 * * `./cbor.ts`     the Plutus/CBOR primitives the two codec modules share
 * * `./datum.ts`    the channel state (`Datum`) and its entry points
 * * `./redeemer.ts` the transition proof (`Redeemer`) and its entry points
 */

export * from "./cbor.js";
export * from "./datum.js";
export * from "./redeemer.js";

/** Re-exported so there is one `ChannelTag`/`Lovelace` in the codebase. */
export {
  Lovelace,
  cbor2Ed25519VerificationKeyCodec,
} from "@konduit/konduit-consumer/cardano";
export { ChannelTag, Index } from "@konduit/konduit-consumer/channel";
export type { Ed25519VerificationKey } from "@konduit/cardano-keys";
export type { Milliseconds } from "@konduit/konduit-consumer/time/duration";
