use std::fs;
use std::path::{Path, PathBuf};

use tonic_prost_build::configure;

pub const ENV_CARGO_FEATURES: &str = "BLN_CLIENT_CARGO_FEATURES";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // INFO :: This `build.rs` introduced to "trick" rust analyzer
    // to analyze code behind feature flags.
    println!("cargo::rerun-if-env-changed={ENV_CARGO_FEATURES}");
    if let Ok(features) = std::env::var(ENV_CARGO_FEATURES) {
        for feature in features.split(":").filter(|x| !x.is_empty()) {
            println!("cargo::rustc-cfg=features=\"{}\"", feature.to_lowercase());
        }
    }

    let proto_root = PathBuf::from("proto");
    let mut protos = Vec::new();

    // Recursively find all .proto files using only std::fs
    collect_protos(&proto_root, &mut protos)?;
    println!("{:?}", protos);

    if !protos.is_empty() {
        configure()
            .build_server(false)
            .compile_protos(&protos, &[proto_root])?;
    }

    println!("cargo:rerun-if-changed=proto");
    Ok(())
}

fn collect_protos(dir: &Path, protos: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                collect_protos(&path, protos)?;
            } else if path.extension().map_or(false, |ext| ext == "proto") {
                protos.push(path);
            }
        }
    }
    Ok(())
}
