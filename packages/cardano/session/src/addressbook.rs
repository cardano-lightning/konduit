//! Label <-> `Address<Shelley>` lookup, unique in both directions.
//! `entries` is the source of truth and the only thing persisted;
//! `by_address` is a derived reverse index kept in sync by insert/remove.

use std::{collections::BTreeMap, path::Path};

use cardano_sdk::{Address, address::kind::Shelley};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct Addressbook {
    entries: BTreeMap<String, Address<Shelley>>,
    #[serde(skip)]
    by_address: BTreeMap<String, String>,
}

impl Default for Addressbook {
    fn default() -> Self {
        Self::try_from(BTreeMap::new()).expect("empty map can't collide")
    }
}

impl TryFrom<BTreeMap<String, Address<Shelley>>> for Addressbook {
    type Error = Error;

    /// Derives `by_address` from `entries` - errors if two labels map to
    /// the same address.
    fn try_from(entries: BTreeMap<String, Address<Shelley>>) -> Result<Self, Error> {
        let mut by_address = BTreeMap::new();
        for (label, address) in &entries {
            let addr_key = address.to_string();
            if let Some(existing) = by_address.insert(addr_key.clone(), label.clone()) {
                return Err(Error::AddressTaken {
                    address: addr_key,
                    existing,
                    new: label.clone(),
                });
            }
        }
        Ok(Addressbook {
            entries,
            by_address,
        })
    }
}

impl<'de> Deserialize<'de> for Addressbook {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct OnDisk {
            entries: BTreeMap<String, Address<Shelley>>,
        }
        let OnDisk { entries } = OnDisk::deserialize(deserializer)?;
        Addressbook::try_from(entries).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("already exists")]
    AlreadyExists,
    #[error("label {0:?} already in use")]
    LabelTaken(String),
    #[error("no entry for label {0:?}")]
    NotFound(String),
    #[error("address {address} already labelled {existing:?}, can't also be {new:?}")]
    AddressTaken {
        address: String,
        existing: String,
        new: String,
    },
    #[error("{0:?} is not a known label or a valid address")]
    Unresolved(String),
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("reading addressbook at {path} - run `addressbook add` to create one?")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parsing addressbook at {path}")]
    Parse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("serializing addressbook")]
    Serialize(#[source] serde_json::Error),
    #[error("writing addressbook to {path}")]
    Write {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl Addressbook {
    pub fn insert(&mut self, label: String, address: Address<Shelley>) -> Result<(), Error> {
        if let Some(existing) = self.entries.get(&label) {
            if *existing == address {
                return Err(Error::AlreadyExists);
            } else {
                return Err(Error::LabelTaken(label));
            }
        }
        let addr_key = address.to_string();
        if let Some(existing) = self.by_address.get(&addr_key) {
            return Err(Error::AddressTaken {
                address: addr_key,
                existing: existing.clone(),
                new: label,
            });
        }
        self.entries.insert(label.clone(), address);
        self.by_address.insert(addr_key, label);
        Ok(())
    }

    pub fn remove(&mut self, label: &str) -> Result<Address<Shelley>, Error> {
        let address = self
            .entries
            .remove(label)
            .ok_or_else(|| Error::NotFound(label.to_string()))?;
        self.by_address.remove(&address.to_string());
        Ok(address)
    }

    /// The removal-side counterpart to `get_label`: drops whatever label
    /// is attached to `address`, if any. `None` (a no-op) if `address`
    /// isn't labelled - unlike `remove`, this isn't an error, since
    /// "already unlabelled" is a perfectly normal thing to ask for.
    pub fn remove_by_address(&mut self, address: &Address<Shelley>) -> Option<String> {
        let label = self.get_label(address)?;
        self.entries.remove(&label);
        self.by_address.remove(&address.to_string());
        Some(label)
    }

    pub fn get(&self, label: &str) -> Option<&Address<Shelley>> {
        self.entries.get(label)
    }

    pub fn get_label(&self, address: &Address<Shelley>) -> Option<String> {
        self.by_address.get(&address.to_string()).cloned()
    }

    /// A known label takes priority; otherwise `input` is parsed as a
    /// literal address. The one place "label or address" is decided.
    pub fn resolve(&self, input: &str) -> Result<Address<Shelley>, Error> {
        if let Some(address) = self.get(input) {
            return Ok(address.clone());
        }
        input
            .parse()
            .map_err(|_| Error::Unresolved(input.to_string()))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Address<Shelley>)> {
        self.entries.iter().map(|(l, a)| (l.as_str(), a))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Replaces string leaves in `value` matching a known address with
    /// its label - exact string matches only, no address-parse attempts.
    #[cfg(feature = "cli")]
    pub fn relabel(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => {
                if let Some(label) = self.by_address.get(s.as_str()) {
                    *s = label.clone();
                }
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(|v| self.relabel(v)),
            serde_json::Value::Object(map) => map.values_mut().for_each(|v| self.relabel(v)),
            _ => {}
        }
    }

    #[cfg(feature = "cli")]
    pub fn load(path: &Path) -> Result<Self, StoreError> {
        match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).map_err(|source| StoreError::Parse {
                path: path.display().to_string(),
                source,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(source) => Err(StoreError::Read {
                path: path.display().to_string(),
                source,
            }),
        }
    }

    #[cfg(feature = "cli")]
    pub fn save(&self, path: &Path) -> Result<(), StoreError> {
        let contents = serde_json::to_string_pretty(self).map_err(StoreError::Serialize)?;
        std::fs::write(path, contents).map_err(|source| StoreError::Write {
            path: path.display().to_string(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cardano_sdk::{Credential, Hash, NetworkId};

    fn mk_address(bytes: Vec<u8>) -> Address<Shelley> {
        Address::new(
            NetworkId::MAINNET,
            Credential::from_key(Hash::<28>::new(bytes)),
        )
    }

    #[test]
    fn insert_rejects_duplicate_label() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();
        let other_addr = mk_address("bob".as_bytes().to_vec());
        let err = book.insert(label.into(), other_addr).unwrap_err();
        assert!(matches!(err, Error::LabelTaken(_)));
    }

    #[test]
    fn insert_rejects_address_already_under_another_label() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();
        let err = book.insert("alice-cold".into(), addr).unwrap_err();
        assert!(matches!(err, Error::AddressTaken { .. }));
    }

    #[test]
    fn remove_prunes_reverse_index_entry() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();
        book.remove(label).unwrap();
        assert_eq!(book.get_label(&addr), None);
    }

    #[test]
    fn remove_by_address_prunes_both_directions() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();
        assert_eq!(book.remove_by_address(&addr), Some(label.to_string()));
        assert_eq!(book.get_label(&addr), None);
        assert_eq!(book.get(label), None);
    }

    #[test]
    fn remove_by_address_is_noop_for_unlabelled_address() {
        let mut book = Addressbook::default();
        let addr = mk_address("bob".as_bytes().to_vec());
        assert_eq!(book.remove_by_address(&addr), None);
    }

    #[test]
    fn deserialize_rebuilds_reverse_index() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();
        let json = serde_json::to_string(&book).unwrap();
        let reloaded: Addressbook = serde_json::from_str(&json).unwrap();
        assert_eq!(reloaded.get_label(&addr), Some("alice".to_string()));
    }

    #[test]
    fn resolve_prefers_label_over_address_parse() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();
        assert_eq!(book.resolve("alice").unwrap(), addr);
    }

    #[test]
    fn resolve_falls_back_to_parsing_a_literal_address() {
        let book = Addressbook::default();
        let addr = mk_address("bob".as_bytes().to_vec());
        assert_eq!(book.resolve(&addr.to_string()).unwrap(), addr);
    }

    #[test]
    fn resolve_rejects_unknown_non_address_input() {
        let book = Addressbook::default();
        assert!(matches!(
            book.resolve("not-a-label-or-address"),
            Err(Error::Unresolved(_))
        ));
    }

    #[test]
    #[cfg(feature = "cli")]
    fn relabel_replaces_matching_address_string() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();

        let mut value = serde_json::json!(addr.to_string());
        book.relabel(&mut value);
        assert_eq!(value, serde_json::json!("alice"));
    }

    #[test]
    #[cfg(feature = "cli")]
    fn relabel_leaves_unknown_string_untouched() {
        let book = Addressbook::default();

        let mut value = serde_json::json!("not an address we know");
        book.relabel(&mut value);
        assert_eq!(value, serde_json::json!("not an address we know"));
    }

    #[test]
    #[cfg(feature = "cli")]
    fn relabel_recurses_into_arrays_and_objects() {
        let mut book = Addressbook::default();
        let label = "alice";
        let addr = mk_address(label.as_bytes().to_vec());
        book.insert(label.into(), addr.clone()).unwrap();

        let mut value = serde_json::json!({
            "utxos": [
                { "address": addr.to_string(), "amount": 5 },
                { "address": "unknown-address", "amount": 10 },
            ],
        });
        book.relabel(&mut value);
        assert_eq!(
            value,
            serde_json::json!({
                "utxos": [
                    { "address": "alice", "amount": 5 },
                    { "address": "unknown-address", "amount": 10 },
                ],
            })
        );
    }

    #[test]
    #[cfg(feature = "cli")]
    fn relabel_is_noop_on_non_string_scalars() {
        let book = Addressbook::default();

        let mut value = serde_json::json!({ "amount": 5, "confirmed": true, "note": null });
        let before = value.clone();
        book.relabel(&mut value);
        assert_eq!(value, before);
    }
}
