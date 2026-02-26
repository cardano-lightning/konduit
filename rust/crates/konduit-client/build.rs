fn main() {
    #[cfg(all(not(feature = "wasm"), not(feature = "reqwest")))]
    compile_error!(
        r#"No platform target selected; please enable either the `reqwest` or `wasm` feature.

    If you're building a desktop application or command-line, you likely want:

        -F reqwest

    If you're building for the browser, you likely want:

        -F wasm"#
    );

    #[cfg(all(feature = "wasm", feature = "reqwest"))]
    compile_error!("Features `reqwest` and `wasm` are mutually exclusive. Enable only one.");
}
