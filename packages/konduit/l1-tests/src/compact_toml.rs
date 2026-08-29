use serde::Serialize;

/// Anything `collapse` can be pointed at: a bare `Table`, or a whole
/// `DocumentMut` (walked via its root table).
pub trait Collapsible {
    fn as_table_mut(&mut self) -> &mut toml_edit::Table;
}

impl Collapsible for toml_edit::Table {
    fn as_table_mut(&mut self) -> &mut toml_edit::Table {
        self
    }
}

impl Collapsible for toml_edit::DocumentMut {
    fn as_table_mut(&mut self) -> &mut toml_edit::Table {
        toml_edit::DocumentMut::as_table_mut(self)
    }
}

/// Renders `value` via normal `toml::to_string_pretty`, then walks the
/// resulting tree collapsing a table (or array-of-tables) to inline form
/// wherever `collapse_me` returns true for its full path from the root.
/// Path segments are bare key names, e.g. `["scenario", "txs"]` for the
/// `txs` field under `[scenario]`, `["scenario", "txs", "note"]` for a
/// nested table inside one `txs` entry.
pub fn pretty_compact_with<T: Serialize>(
    value: &T,
    collapse_me: impl Fn(&[String]) -> bool,
) -> anyhow::Result<String> {
    let mut doc = toml::to_string_pretty(value)?
        .parse::<toml_edit::Document<_>>()?
        .into_mut();
    let mut path = Vec::new();
    collapse_with(doc.as_table_mut(), &mut path, &collapse_me);
    Ok(doc.to_string())
}

fn collapse_with(
    table: &mut toml_edit::Table,
    path: &mut Vec<String>,
    collapse_me: &impl Fn(&[String]) -> bool,
) {
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    for k in keys {
        path.push(k.clone());
        match table.get_mut(&k) {
            Some(toml_edit::Item::Table(t)) => {
                collapse_with(t, path, collapse_me);
                if collapse_me(path) {
                    let inline = t.clone().into_inline_table();
                    table.insert(
                        &k,
                        toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)),
                    );
                    if let Some(mut key) = table.key_mut(&k) {
                        key.leaf_decor_mut().clear();
                    }
                }
            }
            Some(toml_edit::Item::ArrayOfTables(arr)) => {
                // Entries share this key's path — an array index isn't a
                // TOML key, so nested tables inside each entry are still
                // addressed as [..., "txs", "field"], not [..., "txs", "0", "field"].
                for t in arr.iter_mut() {
                    collapse_with(t, path, collapse_me);
                }
                if collapse_me(path) {
                    let mut array = toml_edit::Array::new();
                    for t in arr.iter() {
                        array.push(t.clone().into_inline_table());
                    }
                    table.insert(&k, toml_edit::Item::Value(toml_edit::Value::Array(array)));
                    if let Some(mut key) = table.key_mut(&k) {
                        key.leaf_decor_mut().clear();
                    }
                }
            }
            _ => {}
        }
        path.pop();
    }
}
