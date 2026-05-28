pub const ENV_CARGO_FEATURES: &str = "BLN_CLIENT_CARGO_FEATURES";

// INFO :: This `build.rs` introduced to "trick" rust analyzer
// to analyze code behind feature flags.
fn main() {
    println!("cargo::rerun-if-env-changed={ENV_CARGO_FEATURES}");
    if let Ok(features) = std::env::var(ENV_CARGO_FEATURES) {
        for feature in features.split(":").filter(|x| !x.is_empty()) {
            println!("cargo::rustc-cfg=features=\"{}\"", feature.to_lowercase());
        }
    }
}
