#[cfg(test)]
mod konduit_api_tests {
    use reqwest::{Client, StatusCode};

    use crate::models::*;
    const BASE_URL: &str = "http://127.0.0.1:4444";

    /// Tests the GET /constants endpoint
    #[tokio::test]
    async fn test_get_constants() {
        let client = Client::new();
        let url = format!("{}/constants", BASE_URL);

        let response = client
            .get(&url)
            .send()
            .await
            .expect("Failed to send request to /constants");

        assert_eq!(response.status(), StatusCode::OK);

        // Check content-type header
        assert_eq!(
            response.headers().get("Content-Type").unwrap(),
            "application/json;charset=utf-8"
        );

        // Deserialize the response
        let constants = response
            .json::<Constants>()
            .await
            .expect("Failed to parse Constants JSON");

        // Validate fields based on OpenAPI spec
        assert_eq!(constants.adaptor_key.len(), 64);
        assert!(
            constants.close_period > 0,
            "Close period should be positive"
        );
    }

    /// Tests the POST /quote endpoint (happy path)
    #[tokio::test]
    async fn test_post_quote() {
        let client = Client::new();
        let url = format!("{}/quote", BASE_URL);

        // Mock data based on examples in OpenAPI spec
        let mock_quote_body = QuoteBody {
            consumer_key: <[u8;32]>::try_from(hex::decode("3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29").unwrap()).unwrap(),
            tag: hex::decode("0101010101010101").unwrap(),
            invoice: "lnbcrt1u1p5v9a9cpp5z28n9ewfmc40em7kayfxz5rhy9xdfq9dun6930h6hn4hl05u2e8sdqqcqzzsxqyz5vqsp58yuyhnwywacaj5wj99at99gkwzqg0vej87n25sc8f7yp3qya8jus9qxpqysgq8yp025nppkn3aty2g3g0qun7d8yfe03xtuscy0ns75wc5ny5uvuy36t2m46e2ns88g6deesj87reeuhqm5nzyard49p0a0ys6s8wm3gqrgssv5".to_string(),
        };

        let response = client
            .post(&url)
            .json(&mock_quote_body)
            .send()
            .await
            .expect("Failed to send request to /quote");

        assert_eq!(response.status(), StatusCode::OK);

        // Deserialize the response
        let quote_response = response
            .json::<QuoteResponse>()
            .await
            .expect("Failed to parse QuoteResponse JSON");

        // Validate fields based on OpenAPI spec
        assert_eq!(quote_response.lock.len(), 64);
        assert_eq!(quote_response.recipient.len(), 66);
        assert_eq!(quote_response.payment_addr.len(), 64);
        assert!(quote_response.amount > 0);
        assert!(quote_response.amount_msat > 0);
    }

    /// Tests the POST /pay endpoint (happy path)
    #[tokio::test]
    async fn test_post_pay() {
        let client = Client::new();
        let url = format!("{}/pay", BASE_URL);

        // Mock data based on examples in OpenAPI spec
        // NOTE: For a real test, this data (especially cheque_body and signature)
        // would need to be valid and likely generated from a preceding /quote response.
        let mock_pay_body = PayBody {
            consumer_key: <[u8;32]>::try_from(hex::decode("3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29").unwrap()).unwrap(),
            tag: hex::decode("0101010101010101").unwrap(),
            cheque_body: hex::decode("9f011903e71a00989680582031ad3d39ab3aec5d290d66afbf1efebde22219c39bba73a3ccd1216400212188ff").unwrap(),
            signature: <[u8;64]>::try_from(hex::decode("5840430f32cc2ebad51dd82b3c9c2c868327584bc8edf928cc082c857b4b64b665acc4257cc6b0261b0227c53d2b0f0b7e2e0a549d604834f4fb7ba91cea26d53").unwrap()).unwrap(),
            recipient: <[u8;33]>::try_from(hex::decode("02768dd2ab5682fa1bcd1bbd9f5eb8f47291edfb2ad225b44bb10eafc33a1da80b").unwrap()).unwrap(),
            amount_msat: 100000, // Example value
            payment_addr: <[u8;32]>::try_from(hex::decode("bcd1bbd2768dd2ab5682fa19f5eb8f47291ed44bb10eafc33a1da80bfb2ad225b").unwrap()).unwrap(),
        };

        let response = client
            .post(&url)
            .json(&mock_pay_body)
            .send()
            .await
            .expect("Failed to send request to /pay");

        assert_eq!(response.status(), StatusCode::OK);

        // Deserialize the response
        let receipt = response
            .json::<Receipt>()
            .await
            .expect("Failed to parse Receipt JSON");

        // Basic validation
        assert!(!receipt.squash_body.is_empty());
        assert!(!receipt.signature.is_empty());
        // A successful payment should ideally return at least one unlocked cheque
        assert!(!receipt.unlockeds.is_empty());
        assert!(!receipt.unlockeds[0].secret.is_empty());
    }

    /// Tests the POST /squash endpoint (happy path)
    #[tokio::test]
    async fn test_post_squash() {
        let client = Client::new();
        let url = format!("{}/squash", BASE_URL);

        // Mock data based on examples
        let mock_squash_body = SquashBody {
            consumer_key: <[u8;32]>::try_from(hex::decode("3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29").unwrap()).unwrap(),
            tag: hex::decode("0101010101010101").unwrap(),
            squash_body: vec![],
            signature: <[u8;64]>::try_from(hex::decode("5840430f32cc2ebad51dd82b3c9c2c868327584bc8edf928cc082c857b4b64b665acc4257cc6b0261b0227c53d2b0f0b7e2e0a549d604834f4fb7ba91cea26d5").unwrap()).unwrap(),
        };

        let response = client
            .post(&url)
            .json(&mock_squash_body)
            .send()
            .await
            .expect("Failed to send request to /squash");

        // The spec allows for 200 (with a receipt body) or 202 (empty body)
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::ACCEPTED,
            "Expected status 200 or 202, got {}",
            status
        );

        if status == StatusCode::OK {
            // If 200, we expect a Receipt body
            let receipt = response
                .json::<Receipt>()
                .await
                .expect("Failed to parse Receipt JSON from /squash");
            assert!(!receipt.squash_body.is_empty());
        } else if status == StatusCode::ACCEPTED {
            // If 202, we expect an empty body
            let body = response
                .text()
                .await
                .expect("Failed to get text from /squash 202 response");
            assert!(body.is_empty(), "Expected empty body for 202 response");
        }
    }
}
