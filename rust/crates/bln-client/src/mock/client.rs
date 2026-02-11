use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::api::Api;
use crate::{PayRequest, PayResponse, QuoteRequest, QuoteResponse, RevealRequest, RevealResponse};

#[derive(Debug, Clone, Default)]
pub struct Client {
    /// Internal storage for secrets: Map<Lock, Secret>
    secrets: Arc<Mutex<HashMap<[u8; 32], [u8; 32]>>>,
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    /// Preload or dynamically add a secret to the mock's internal state.
    ///
    /// # Arguments
    /// * `lock` - The 32-byte lock (often a payment hash)
    /// * `secret` - The 32-byte secret (often a preimage)
    pub fn add_secret(&self, lock: [u8; 32], secret: [u8; 32]) {
        let mut secrets = self.secrets.lock().unwrap();
        secrets.insert(lock, secret);
    }

    /// Convenience method to bulk-load secrets.
    pub fn load_secrets(&self, items: Vec<([u8; 32], [u8; 32])>) {
        let mut secrets = self.secrets.lock().unwrap();
        for (lock, secret) in items {
            secrets.insert(lock, secret);
        }
    }
}

#[async_trait]
impl Api for Client {
    async fn quote(&self, _quote_request: QuoteRequest) -> crate::Result<QuoteResponse> {
        Ok(QuoteResponse {
            relative_timeout: Duration::from_secs(3600),
            fee_msat: 1000,
        })
    }

    async fn pay(&self, req: PayRequest) -> crate::Result<PayResponse> {
        let lock = req.invoice.payment_hash;
        self.reveal(RevealRequest { lock }).await
    }

    async fn reveal(&self, req: RevealRequest) -> crate::Result<RevealResponse> {
        let secrets = self.secrets.lock().expect("mutex poisoned");
        let secret = secrets.get(&req.lock).cloned();

        Ok(RevealResponse { secret })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Api;
    use crate::mock::client::Client;
    use crate::{QuoteRequest, RevealRequest};
    use std::time::Duration;

    // Helper to create a dummy 32-byte array
    fn bytes32(val: u8) -> [u8; 32] {
        [val; 32]
    }

    #[tokio::test]
    async fn test_quote_returns_default_values() {
        let client = Client::new();
        let req = QuoteRequest {
            amount_msat: 5000,
            payee: [0u8; 33],
        };

        let res = client.quote(req).await.unwrap();

        assert_eq!(res.fee_msat, 1000);
        assert_eq!(res.relative_timeout, Duration::from_secs(3600));
    }

    #[tokio::test]
    async fn test_reveal_with_preloaded_secret() {
        let client = Client::new();
        let lock = bytes32(1);
        let secret = bytes32(42);

        // Preload secret
        client.add_secret(lock, secret);

        let req = RevealRequest { lock };
        let res = client.reveal(req).await.unwrap();

        assert_eq!(res.secret, Some(secret));
    }

    #[tokio::test]
    async fn test_reveal_missing_secret_returns_none() {
        let client = Client::new();
        let lock = bytes32(1);

        let req = RevealRequest { lock };
        let res = client.reveal(req).await.unwrap();

        assert_eq!(res.secret, None);
    }

    #[tokio::test]
    async fn test_dynamic_loading_during_runtime() {
        let client = Client::new();
        let client_clone = client.clone();

        let lock = bytes32(2);
        let secret = bytes32(99);

        // Simulate an async task adding a secret later
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            client_clone.add_secret(lock, secret);
        });

        // Initially it should be None
        let res_initial = client.reveal(RevealRequest { lock }).await.unwrap();
        assert!(res_initial.secret.is_none());

        // Wait for the background task
        tokio::time::sleep(Duration::from_millis(50)).await;

        let res_final = client.reveal(RevealRequest { lock }).await.unwrap();
        assert_eq!(res_final.secret, Some(secret));
    }

    #[tokio::test]
    async fn test_bulk_loading_secrets() {
        let client = Client::new();
        let secrets = vec![(bytes32(10), bytes32(100)), (bytes32(20), bytes32(200))];

        client.load_secrets(secrets);

        let res1 = client
            .reveal(RevealRequest { lock: bytes32(10) })
            .await
            .unwrap();
        let res2 = client
            .reveal(RevealRequest { lock: bytes32(20) })
            .await
            .unwrap();

        assert_eq!(res1.secret, Some(bytes32(100)));
        assert_eq!(res2.secret, Some(bytes32(200)));
    }
}
