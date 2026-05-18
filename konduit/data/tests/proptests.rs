//! CBOR ↔ CDDL correspondence tests.
//!
//! For every public wire type in konduit-data:
//!   1. Generate an arbitrary instance (proptest).
//!   2. Encode it to CBOR using the same codec used on-chain.
//!   3. Validate the CBOR bytes against the type's CDDL rule in the full spec.
//!
//! # Cheque / Receipt handling
//!
//! `cddl-cat` 0.7 does not parse `#6.N(type)` CBOR-tag notation.  The generated
//! spec correctly emits `cheque = #6.121(unlocked) / #6.122(locked)`, but tests
//! use a patched spec where `cheque = any` so that cddl-cat can still validate
//! every other rule (including `receipt`, which contains `[* cheque]`).
//!
//! Cheque tests instead strip the outer CBOR tag byte and validate the inner
//! indefinite array against the appropriate `unlocked` / `locked` rule.
//!
//! Run with:
//!   cargo test -p konduit-data --features cddl,proptest

mod cddl_cbor {
    use cardano_sdk::{PlutusData, cbor::ToCbor};
    use cddl_cat::validate_cbor_bytes;
    use cuddly::{CddlSpec, ToCddl};
    use proptest::prelude::*;

    use konduit_data::{
        Cheque, ChequeBody, Duration, Indexes, Keytag, Lock, Locked, Pending, Receipt, Secret,
        Squash, SquashBody, Tag, Unlocked, Unpend, Used,
    };

    // ---------------------------------------------------------------------------
    // Full spec (used by the binary) and validation spec (cddl-cat compatible)
    // ---------------------------------------------------------------------------

    #[derive(CddlSpec)]
    #[cddl_spec(types(
        Duration, Lock, Secret, Tag, Keytag, Unpend, Indexes, Used, ChequeBody, Pending,
        SquashBody, Locked, Unlocked, Squash, Cheque, Receipt,
    ))]
    struct KonduitDataSpec;

    /// The CDDL spec patched for cddl-cat 0.7 compatibility:
    ///
    /// - `cheque = any` — cddl-cat can't parse `#6.N(type)` tag notation.
    /// - `receipt = [squash, any]` — cddl-cat's CBOR parser (ciborium) returns
    ///   `Value::Hidden` for CBOR-tagged items (Plutus constrs) nested inside
    ///   arrays; replacing `[* cheque]` with `any` avoids that path while still
    ///   fully validating the `squash` field and outer array structure.
    fn validation_spec() -> String {
        let spec = KonduitDataSpec::cddl();
        let spec = replace_rule(&spec, "cheque", "cheque = any");
        replace_rule(&spec, "receipt", "receipt = [squash, any]")
    }

    /// Replace a named CDDL rule block (possibly multi-line, terminated by a
    /// blank line) with `replacement`.  Rule blocks are double-newline separated.
    fn replace_rule(spec: &str, rule_name: &str, replacement: &str) -> String {
        let prefix = format!("{rule_name} =");
        let mut out = String::with_capacity(spec.len());
        let mut skip = false;
        for line in spec.lines() {
            if line.starts_with(&prefix) {
                out.push_str(replacement);
                out.push('\n');
                skip = true;
            } else if skip {
                if line.trim().is_empty() {
                    out.push('\n');
                    skip = false;
                }
                // drop interior lines of the replaced rule
            } else {
                out.push_str(line);
                out.push('\n');
            }
        }
        out
    }

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    /// Validate `cbor` against `T`'s CDDL rule using the cddl-cat-compatible spec.
    fn validate<T: ToCddl>(cbor: &[u8]) {
        let spec = validation_spec();
        let rule = T::cddl_ref();
        validate_cbor_bytes(&rule, &spec, cbor)
            .unwrap_or_else(|e| panic!("CDDL validation failed for `{rule}`:\n  {e:?}"));
    }

    /// Encode a type to CBOR via its PlutusData representation.
    fn plutus_cbor<'a, T: Into<PlutusData<'a>>>(val: T) -> Vec<u8> {
        val.into().to_cbor()
    }

    /// Validate a Cheque's CBOR by stripping the outer Plutus constructor tag
    /// and validating the inner indefinite array against `unlocked` or `locked`.
    ///
    /// Plutus constr 0 → CBOR tag 121 (0xD8 0x79), constr 1 → tag 122 (0xD8 0x7A).
    fn validate_cheque_cbor(cbor: &[u8]) {
        let spec = validation_spec();
        assert!(cbor.len() >= 2, "CBOR too short for a Plutus constructor");
        assert_eq!(
            cbor[0], 0xD8,
            "expected CBOR tag prefix byte 0xD8 (major type 6, 1-byte argument)"
        );
        let (rule, inner) = match cbor[1] {
            0x79 => ("unlocked", &cbor[2..]), // tag 121 → Cheque::Unlocked
            0x7A => ("locked", &cbor[2..]),   // tag 122 → Cheque::Locked
            t => panic!("unexpected Plutus constr tag byte: 0x{t:02X}"),
        };
        validate_cbor_bytes(rule, &spec, inner)
            .unwrap_or_else(|e| panic!("Cheque inner ({rule}) CDDL validation failed:\n  {e:?}"));
    }

    // ---------------------------------------------------------------------------
    // Base types (bytes / uint)
    // ---------------------------------------------------------------------------

    proptest! {
        #[test]
        fn duration_cbor(val in any::<Duration>()) {
            validate::<Duration>(&plutus_cbor(val));
        }

        #[test]
        fn lock_cbor(val in any::<Lock>()) {
            validate::<Lock>(&plutus_cbor(val));
        }

        #[test]
        fn secret_cbor(val in any::<Secret>()) {
            validate::<Secret>(&plutus_cbor(val));
        }

        #[test]
        fn tag_cbor(val in any::<Tag>()) {
            validate::<Tag>(&plutus_cbor(val));
        }

        #[test]
        fn keytag_cbor(val in any::<Keytag>()) {
            validate::<Keytag>(&plutus_cbor(val));
        }

        #[test]
        fn unpend_cbor(val in any::<Unpend>()) {
            validate::<Unpend>(&plutus_cbor(val));
        }
    }

    // ---------------------------------------------------------------------------
    // Indexes: [* uint]
    // ---------------------------------------------------------------------------

    proptest! {
        #[test]
        fn indexes_cbor(val in any::<Indexes>()) {
            // From<&Indexes> for PlutusData (reference only)
            validate::<Indexes>(&PlutusData::from(&val).to_cbor());
        }
    }

    // ---------------------------------------------------------------------------
    // Struct types: CBOR indefinite arrays
    // ---------------------------------------------------------------------------

    proptest! {
        #[test]
        fn used_cbor(val in any::<Used>()) {
            validate::<Used>(&plutus_cbor(val));
        }

        #[test]
        fn cheque_body_cbor(val in any::<ChequeBody>()) {
            validate::<ChequeBody>(&plutus_cbor(val));
        }

        #[test]
        fn pending_cbor(val in any::<Pending>()) {
            validate::<Pending>(&plutus_cbor(val));
        }

        #[test]
        fn squash_body_cbor(val in any::<SquashBody>()) {
            validate::<SquashBody>(&plutus_cbor(val));
        }

        #[test]
        fn locked_cbor(val in any::<Locked>()) {
            validate::<Locked>(&plutus_cbor(val));
        }

        #[test]
        fn unlocked_cbor(val in any::<Unlocked>()) {
            validate::<Unlocked>(&plutus_cbor(val));
        }

        #[test]
        fn squash_cbor(val in any::<Squash>()) {
            validate::<Squash>(&plutus_cbor(val));
        }
    }

    // ---------------------------------------------------------------------------
    // Cheque: Plutus constr (CBOR tag 121 / 122)
    //
    // Strip the outer tag; validate inner indefinite array against unlocked/locked.
    // ---------------------------------------------------------------------------

    proptest! {
        #[test]
        fn cheque_cbor(val in any::<Cheque>()) {
            validate_cheque_cbor(&plutus_cbor(val));
        }
    }

    // ---------------------------------------------------------------------------
    // Receipt: minicbor outer array, PlutusData inner fields
    //
    // cddl-cat's serde_cbor backend returns Value::Hidden for CBOR-tagged items
    // (Plutus constrs) even when matched against `any`, so full receipt validation
    // only works when cheques is empty. For non-empty receipts we validate the
    // squash field and each cheque component individually instead.
    // ---------------------------------------------------------------------------

    proptest! {
        #[test]
        fn receipt_cbor(val in any::<Receipt>()) {
            if val.cheques.is_empty() {
                // Full structural validation: outer 2-element array + squash rule
                let cbor = minicbor::to_vec(&val).expect("minicbor encoding failed");
                validate::<Receipt>(&cbor);
            } else {
                // Per-component validation: squash + each cheque
                validate::<Squash>(&plutus_cbor(val.squash.clone()));
                for cheque in &val.cheques {
                    validate_cheque_cbor(&plutus_cbor(cheque.clone()));
                }
            }
        }
    }
}
