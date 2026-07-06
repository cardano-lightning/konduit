use crate::types::{
    BulkPatternsBody, Datum, Deleted, ForcedRollback, ForcedRollbackBody, Health, Match,
    MatchFilters, MatchStatus, Metadata, Pattern, Point, PutPatternBody, Script,
};
use crate::{Error, Result};
use reqwest::{Client as HttpClient, Method, Url};
use std::fmt::Write as _;

/// A client for talking to a Kupo HTTP server.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: Url,
    http: HttpClient,
}

impl Client {
    /// Create a new client targeting the given base URL (e.g. `http://localhost:1442`).
    pub fn new(base_url: &str) -> Result<Self> {
        let base_url = Url::parse(base_url.trim_end_matches('/'))
            .map_err(|e| Error::InvalidUrl(format!("{e}: {base_url}")))?;

        Ok(Self {
            base_url,
            http: HttpClient::new(),
        })
    }

    /// The base URL this client targets, without any trailing slash.
    pub fn base_url(&self) -> &str {
        self.base_url.as_str()
    }

    // ---------------------------------------------------------------- Health

    /// Retrieve the server's application health status.
    ///
    /// See [`Health`] for the various response codes the server may return.
    pub async fn health(&self) -> Result<Health> {
        self.get::<Health>("/health", &[]).await
    }

    // ---------------------------------------------------------------- Matches

    /// Retrieve all matches from the database.
    pub async fn all_matches(&self, filters: &MatchFilters) -> Result<Vec<Match>> {
        let query = filters.to_query_items();
        self.get::<Vec<Match>>("/matches", &query).await
    }

    /// Retrieve matches from the database matching the given pattern.
    pub async fn matches(&self, pattern: &Pattern, filters: &MatchFilters) -> Result<Vec<Match>> {
        let path = format!("/matches/{}", urlencoded(pattern));
        let query = filters.to_query_items();
        self.get::<Vec<Match>>(&path, &query).await
    }

    /// Delete all matches matching the given pattern.
    ///
    /// Only allowed if the provided pattern is not currently active.
    pub async fn delete_matches(&self, pattern: &Pattern) -> Result<Deleted> {
        let path = format!("/matches/{}", urlencoded(pattern));
        self.delete::<Deleted>(&path).await
    }

    // ---------------------------------------------------------------- Datums

    /// Retrieve a datum by its blake2b-256 hash digest.
    ///
    /// Returns `None` when the datum is unknown to the server.
    pub async fn datum(&self, datum_hash: &crate::Hash32) -> Result<Option<Datum>> {
        let path = format!("/datums/{}", datum_hash.0);
        self.get_nullable::<Datum>(&path, &[]).await
    }

    // ---------------------------------------------------------------- Scripts

    /// Retrieve a script by its blake2b-224 hash digest.
    ///
    /// Returns `None` when the script is unknown to the server.
    pub async fn script(&self, script_hash: &crate::Hash28) -> Result<Option<Script>> {
        let path = format!("/scripts/{}", script_hash.0);
        self.get_nullable::<Script>(&path, &[]).await
    }

    // ---------------------------------------------------------------- Patterns

    /// Retrieve all patterns currently configured on the server.
    pub async fn patterns(&self) -> Result<Vec<Pattern>> {
        self.get::<Vec<Pattern>>("/patterns", &[]).await
    }

    /// Add a new pattern to watch, with a forced rollback.
    pub async fn put_pattern(
        &self,
        pattern: &Pattern,
        forced_rollback: &ForcedRollback,
    ) -> Result<Vec<Pattern>> {
        let path = format!("/patterns/{}", urlencoded(pattern));
        let body = PutPatternBody {
            forced_rollback: ForcedRollbackBody::from(forced_rollback),
        };
        self.put::<_, Vec<Pattern>>(&path, &body).await
    }

    /// Remove a pattern from the database and active filtering.
    pub async fn delete_pattern(&self, pattern: &Pattern) -> Result<Deleted> {
        let path = format!("/patterns/{}", urlencoded(pattern));
        self.delete::<Deleted>(&path).await
    }

    /// Bulk-add many patterns at once.
    pub async fn put_patterns(
        &self,
        patterns: &[Pattern],
        forced_rollback: &ForcedRollback,
    ) -> Result<Vec<Pattern>> {
        let body = BulkPatternsBody {
            patterns,
            forced_rollback: ForcedRollbackBody::from(forced_rollback),
        };
        self.put::<_, Vec<Pattern>>("/patterns", &body).await
    }

    // ------------------------------------------------------------- Checkpoints

    /// Retrieve a sample of all checkpoints currently in the database.
    pub async fn checkpoints(&self) -> Result<Vec<Point>> {
        self.get::<Vec<Point>>("/checkpoints", &[]).await
    }

    /// Retrieve a checkpoint by its slot number.
    ///
    /// By default, the server falls back to the closest checkpoint that is
    /// **before** the given slot. Pass `strict = true` to look for an exact match.
    pub async fn checkpoint(&self, slot_no: u64, strict: bool) -> Result<Option<Point>> {
        let path = format!("/checkpoints/{slot_no}");
        let query: Vec<(String, String)> = if strict {
            vec![("strict".to_string(), String::new())]
        } else {
            Vec::new()
        };
        self.get_nullable::<Point>(&path, &query).await
    }

    // ---------------------------------------------------------------- Metadata

    /// Retrieve all metadata seen in a block at the given slot, optionally
    /// filtered by transaction id.
    pub async fn metadata(
        &self,
        slot_no: u64,
        transaction_id: Option<&crate::Hash32>,
    ) -> Result<Vec<Metadata>> {
        let path = format!("/metadata/{slot_no}");
        let query: Vec<(String, String)> = transaction_id
            .map(|tx| vec![("transaction_id".to_string(), tx.0.clone())])
            .unwrap_or_default();
        self.get::<Vec<Metadata>>(&path, &query).await
    }

    // --------------------------------------------------------------- Plumbing

    fn url(&self, path: &str) -> Result<Url> {
        self.base_url
            .join(path.trim_start_matches('/'))
            .map_err(|e| Error::InvalidUrl(format!("joining {path:?}: {e}")))
    }

    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<T> {
        let url = self.url(path)?;
        let res = self.http.get(url).query(query).send().await?;
        self.parse(res).await
    }

    async fn get_nullable<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(String, String)],
    ) -> Result<Option<T>> {
        let url = self.url(path)?;
        let res = self.http.get(url).query(query).send().await?;
        self.parse_nullable(res).await
    }

    async fn delete<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.url(path)?;
        let res = self.http.delete(url).send().await?;
        self.parse(res).await
    }

    async fn put<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = self.url(path)?;
        let res = self
            .http
            .request(Method::PUT, url)
            .json(body)
            .send()
            .await?;
        self.parse(res).await
    }

    async fn parse<T: serde::de::DeserializeOwned>(&self, res: reqwest::Response) -> Result<T> {
        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body,
            });
        }

        serde_json::from_str(&body).map_err(Error::from)
    }

    async fn parse_nullable<T: serde::de::DeserializeOwned>(
        &self,
        res: reqwest::Response,
    ) -> Result<Option<T>> {
        let status = res.status();
        let body = res.text().await?;

        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                body,
            });
        }

        if body.trim().eq_ignore_ascii_case("null") {
            return Ok(None);
        }

        serde_json::from_str(&body).map(Some).map_err(Error::from)
    }
}

/// Anything that can produce a list of `(name, value)` pairs for the request query string.
trait ToQueryItems {
    fn to_query_items(&self) -> Vec<(String, String)>;
}

impl ToQueryItems for MatchFilters {
    fn to_query_items(&self) -> Vec<(String, String)> {
        let mut params: Vec<(String, String)> = Vec::new();

        if self.resolve_hashes {
            params.push(("resolve_hashes".to_string(), String::new()));
        }
        match self.status {
            MatchStatus::Any => {}
            MatchStatus::Spent => params.push(("spent".to_string(), String::new())),
            MatchStatus::Unspent => params.push(("unspent".to_string(), String::new())),
        }
        params.push(("order".to_string(), self.order.as_str().to_string()));

        if let Some(b) = &self.created_after {
            params.push(("created_after".to_string(), b.to_string()));
        }
        if let Some(b) = &self.spent_after {
            params.push(("spent_after".to_string(), b.to_string()));
        }
        if let Some(b) = &self.created_before {
            params.push(("created_before".to_string(), b.to_string()));
        }
        if let Some(b) = &self.spent_before {
            params.push(("spent_before".to_string(), b.to_string()));
        }
        if let Some(p) = &self.policy_id {
            params.push(("policy_id".to_string(), p.0.clone()));
        }
        if let Some(n) = &self.asset_name {
            params.push(("asset_name".to_string(), n.clone()));
        }
        if let Some(t) = &self.transaction_id {
            params.push(("transaction_id".to_string(), t.0.clone()));
        }
        if let Some(o) = self.output_index {
            params.push(("output_index".to_string(), o.to_string()));
        }

        params
    }
}

/// URL-encode a pattern for use in a path segment.
///
/// Patterns may contain forward slashes (e.g. `addr_vk.../*`), which reqwest's
/// URL parser would otherwise interpret as path separators. The whitelist below
/// corresponds to `pchar` in RFC 3986 (unreserved + sub-delims + `:` / `@`),
/// which is what URL path segments allow.
fn urlencoded(pattern: &Pattern) -> String {
    let mut out = String::with_capacity(pattern.to_string().len());
    for byte in pattern.to_string().bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b':'
            | b'@' => {
                out.push(byte as char);
            }
            _ => {
                let _ = write!(out, "%{:02X}", byte);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AssetNamePattern, MatchBound, Order, Pattern, RollbackTo};

    #[test]
    fn urlencoded_escapes_path_separators() {
        let pat = Pattern::Address("addr_vk1abc/*".to_string());
        let enc = urlencoded(&pat);
        assert_eq!(enc, "addr_vk1abc%2F*");
    }

    #[test]
    fn urlencoded_preserves_safe_characters() {
        let pat = Pattern::OutputReference {
            index: Some(42),
            transaction_id: "a".repeat(64),
        };
        let enc = urlencoded(&pat);
        assert_eq!(enc, format!("42@{}", "a".repeat(64)));
    }

    #[test]
    fn pattern_display_roundtrips() {
        assert_eq!(Pattern::wildcard().to_string(), "*");
        assert_eq!(
            Pattern::address("addr_vk1abc/*").to_string(),
            "addr_vk1abc/*"
        );
        assert_eq!(
            Pattern::AssetId {
                policy_id: "abc".into(),
                asset_name: AssetNamePattern::Wildcard,
            }
            .to_string(),
            "abc.*"
        );
        assert_eq!(
            Pattern::asset("abc", "deadbeef").to_string(),
            "abc.deadbeef"
        );
        assert_eq!(
            Pattern::tx("a".repeat(64).as_str()).to_string(),
            format!("*@{}", "a".repeat(64))
        );
        assert_eq!(
            Pattern::output_ref(7, "a".repeat(64).as_str()).to_string(),
            format!("7@{}", "a".repeat(64))
        );
    }

    #[test]
    fn filters_render_to_expected_query() {
        let filters = MatchFilters::new()
            .with_resolve_hashes(true)
            .with_status(MatchStatus::Unspent)
            .with_order(Order::OldestFirst)
            .with_created_after(MatchBound::Slot(42))
            .with_policy(crate::Hash28("abc".into()))
            .with_asset_name("deadbeef")
            .with_output_index(3);

        let params = filters.to_query_items();
        let map: std::collections::BTreeMap<&str, &str> = params
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        assert_eq!(map.get("resolve_hashes"), Some(&""));
        assert_eq!(map.get("unspent"), Some(&""));
        assert_eq!(map.get("order"), Some(&"oldest_first"));
        assert_eq!(map.get("created_after"), Some(&"42"));
        assert_eq!(map.get("policy_id"), Some(&"abc"));
        assert_eq!(map.get("asset_name"), Some(&"deadbeef"));
        assert_eq!(map.get("output_index"), Some(&"3"));
    }

    #[test]
    fn client_rejects_invalid_base_url() {
        assert!(Client::new("not a url").is_err());
    }

    #[test]
    fn client_strips_trailing_slash() {
        let client = Client::new("http://localhost:1442/").unwrap();
        assert_eq!(client.base_url(), "http://localhost:1442/");
    }

    #[test]
    fn rollback_to_helpers_construct_expected_shape() {
        let slot = RollbackTo::slot(123);
        assert_eq!(slot.slot_no, 123);
        assert!(slot.header_hash.is_none());

        let hash = crate::Hash32("a".repeat(64));
        let point = RollbackTo::point(456, hash.clone());
        assert_eq!(point.slot_no, 456);
        assert_eq!(point.header_hash, Some(hash));
    }
}
