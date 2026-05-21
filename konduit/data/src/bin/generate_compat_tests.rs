use konduit_data::aiken::compat;
use std::{fmt::Write as _, path::PathBuf};

fn main() {
    let out_path: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Default: write next to the kernel lib directory
            let manifest = env!("CARGO_MANIFEST_DIR");
            PathBuf::from(manifest).join("../../kernel/lib/tests/compat.ak")
        });

    let mut out = String::new();

    writeln!(
        out,
        "// GENERATED — do not edit by hand.\n\
         // Re-run: cargo run --features aiken -p konduit-data --bin generate_compat_tests\n\
         //\n\
         // Encoding tests: builtin.serialise_data(<aiken_value>) == #\"<rust_cbor_hex>\"\n\
         // Signing tests:  wellsigned(vk, tag, <body>, sig)\n"
    )
    .unwrap();

    writeln!(out, "use aiken/builtin").unwrap();
    writeln!(out, "use aiken/crypto.{{VerificationKey}}").unwrap();
    writeln!(out, "use konduit/types as t").unwrap();
    writeln!(out, "use konduit/wellformed.{{wellsigned}}").unwrap();
    writeln!(out).unwrap();

    // Collect all (name, body) pairs
    let mut tests: Vec<(String, String)> = Vec::new();
    tests.extend(compat::used_tests());
    tests.extend(compat::pending_tests());
    tests.extend(compat::indexes_tests());
    tests.extend(compat::cheque_body_tests());
    tests.extend(compat::constants_tests());
    tests.extend(compat::stage_tests());
    tests.extend(compat::datum_tests());
    tests.extend(compat::unpend_tests());
    tests.extend(compat::eol_tests());
    tests.extend(compat::redeemer_tests());
    tests.extend(compat::locked_cheque_encoding_tests());
    tests.extend(compat::unlocked_cheque_encoding_tests());
    tests.extend(compat::locked_cheque_signing_tests());
    tests.extend(compat::squash_signing_tests());

    for (name, body) in &tests {
        writeln!(out, "test {name}() {{").unwrap();
        out.push_str(body);
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // Write to file or stdout
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).expect("create output directory");
    }
    std::fs::write(&out_path, &out).expect("write compat.ak");
    eprintln!("wrote {} tests to {}", tests.len(), out_path.display());
}
