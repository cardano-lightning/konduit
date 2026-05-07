# task-100 research

- test-seam finding: the smallest truthful seam for paginated `utxos_at(payment, None)` coverage is an async page-fetch closure around the existing `load_utxos()` loop, not a broad fake client trait; this keeps the real cursor-advance and page-accumulation control flow under test without spreading abstraction into runtime code
- mapping finding: `map_output_data()` precedence between native output bytes and parsed protobuf output is worth locking down explicitly because both sources can coexist in fixtures; the accepted rule remains native bytes first, parsed fallback only when native bytes are absent
- protocol-parameter finding: the current Shelley-boundary reconstruction of `start_time` can be asserted indirectly through `ProtocolParameters::posix_to_slot`, which avoids adding extra crate dependencies only for test inspection while still proving the derived timing model
- submit-error finding: submit-path coverage only needs a small helper around `utxorpc::Error` context wrapping; testing transport-stage details directly would have added unnecessary dependency or mocking surface for this crate
- connector-core network finding: `network_from_genesis()` still intentionally accepts an empty `network_id` when the network magic is recognized, but must reject unsupported magic and inconsistent `network_id` versus magic
