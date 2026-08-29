use anyhow::Result;
use cardano_sdk::{Hash, Signature, SigningKey, VerificationKey};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::str::FromStr;

#[derive(Serialize)]
struct KeyInfo {
    #[serde(with = "hex::serde")]
    sk: [u8; 32], // leaked signing key bytes -- CLI-only, by design
    #[serde(with = "hex::serde")]
    vk: [u8; 32],
    #[serde(with = "hex::serde")]
    vkh: [u8; 28],
}

impl From<SigningKey> for KeyInfo {
    fn from(sk: SigningKey) -> Self {
        let vk = sk.to_verification_key();
        let vkh = <[u8; 28]>::from(Hash::<28>::new(vk));
        KeyInfo {
            vk: vk.into_bytes(),
            vkh,
            sk: unsafe { SigningKey::leak(sk) },
        }
    }
}

#[derive(Parser)]
/// A tiny tool to generate and convert keys.
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print sk/vk/vkh for a given signing key (hex).
    Info {
        #[arg(value_parser = SigningKey::from_str)]
        sk: SigningKey,
    },
    /// Generate a signing key (from a seed, or OS entropy) and print its info.
    Generate {
        /// Use for deterministic keygen; omit for OS entropy.
        seed: Option<String>,
    },
    /// Derive the verification key (hex) for a signing key.
    Vk {
        #[arg(value_parser = SigningKey::from_str)]
        sk: SigningKey,
    },
    /// Hash a verification key -> 28-byte vkh (hex).
    Vkh {
        #[arg(value_parser = VerificationKey::from_str)]
        vk: VerificationKey,
    },
    /// Sign a hex-encoded message with a signing key.
    Sign {
        #[arg(value_parser = SigningKey::from_str)]
        sk: SigningKey,
        #[arg(value_parser = parse_hex)]
        msg: Vec<u8>,
    },
    /// Verify a hex-encoded signature against a vk and message.
    Verify {
        #[arg(value_parser = VerificationKey::from_str)]
        vk: VerificationKey,
        #[arg(value_parser = parse_hex)]
        msg: Vec<u8>,
        #[arg(value_parser = Signature::from_str)]
        sig: Signature,
    },
}

fn parse_hex(s: &str) -> Result<Vec<u8>> {
    Ok(hex::decode(s)?)
}

fn print_json<T: Serialize>(v: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(v)?);
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Info { sk } => print_json(&KeyInfo::from(sk))?,
        Cmd::Generate { seed } => {
            let sk = match seed {
                Some(s) => SigningKey::from(<[u8; 32]>::from(Hash::<32>::new(s))),
                None => SigningKey::new(),
            };
            print_json(&KeyInfo::from(sk))?
        }
        Cmd::Vk { sk } => println!("{}", sk.to_verification_key()),
        Cmd::Vkh { vk } => println!("{}", Hash::<28>::new(vk)),
        Cmd::Sign { sk, msg } => println!("{}", sk.sign(msg)),
        Cmd::Verify { vk, msg, sig } => println!("{}", vk.verify(msg, &sig)),
    }
    Ok(())
}
