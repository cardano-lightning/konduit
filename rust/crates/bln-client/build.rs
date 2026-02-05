fn main() {
    if let Ok(features) = std::env::var("CARGO_FEATURES") {
        for feature in features.split(":").filter(|x| !x.is_empty()) {
            println!("cargo:rustc-cfg=feature=\"{}\"", feature.to_lowercase());
        }
    }
}
