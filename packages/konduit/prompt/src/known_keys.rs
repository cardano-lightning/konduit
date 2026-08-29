use std::collections::BTreeMap;

use konduit_data::{Constants, VerifyingKey};

use crate::prompt::Candidate;

/// Human labels for verifying keys the operator knows about. Keyed by
/// label (assumed unique) — same direction as `Keyring`. `vkey -> label`
/// lookups (used for display) invert on demand via a scan; stays small
/// enough that a second index isn't worth it.
#[derive(Debug, Default, Clone)]
pub struct KnownKeys(BTreeMap<String, VerifyingKey>);

impl KnownKeys {
    pub fn new(keys: BTreeMap<String, VerifyingKey>) -> Self {
        Self(keys)
    }

    pub fn extend(&mut self, entries: BTreeMap<String, VerifyingKey>) {
        self.0.extend(entries);
    }

    pub fn vkey_for(&self, label: &str) -> Option<&VerifyingKey> {
        self.0.get(label)
    }

    pub fn label_for(&self, vkey: &VerifyingKey) -> Option<&str> {
        self.0
            .iter()
            .find(|(_, v)| *v == vkey)
            .map(|(label, _)| label.as_str())
    }

    pub fn label_for_verification_key(&self, vkey: &cardano_sdk::VerificationKey) -> Option<&str> {
        self.0
            .iter()
            .find(|(_, known)| &konduit_tmp::from_verifying_key((*known).clone()) == vkey)
            .map(|(label, _)| label.as_str())
    }

    pub fn channel_label(&self, constants: &Constants) -> Option<String> {
        let mut parts = Vec::new();
        if let Some(l) = self.label_for(&constants.sub_vkey) {
            parts.push(format!("A:{l}"));
        }
        if let Some(l) = self.label_for(&constants.add_vkey) {
            parts.push(format!("C:{l}"));
        }
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    pub fn prettify(&self, vkey: &VerifyingKey) -> String {
        self.label_for(vkey)
            .map(str::to_string)
            .unwrap_or_else(|| short_hex(vkey.as_ref()))
    }

    /// Direct now — storage is already label-keyed, same as what
    /// `Candidate` wants; no inversion needed.
    pub fn candidates(&self) -> Vec<Candidate<VerifyingKey>> {
        self.0
            .iter()
            .map(|(label, vkey)| Candidate::new(label.clone(), vkey.clone()))
            .collect()
    }
}

fn short_hex(bytes: &[u8]) -> String {
    let hex = hex::encode(bytes);
    if hex.len() > 12 {
        format!("{}…{}", &hex[..6], &hex[hex.len() - 4..])
    } else {
        hex
    }
}
