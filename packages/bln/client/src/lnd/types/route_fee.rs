use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};

#[derive(Debug, Serialize)]
pub struct Request {
    pub payment_request: String,
    pub timeout: u32,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Response {
    #[serde_as(as = "DisplayFromStr")]
    pub routing_fee_msat: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub time_lock_delay: u64,
    pub failure_reason: String,
}

#[cfg(test)]
mod tests {
    use super::Response;

    #[test]
    fn decodes_rest_integer_strings() {
        let response: Response = serde_json::from_str(
            r#"{"routing_fee_msat":"526","time_lock_delay":"682","failure_reason":"FAILURE_REASON_NONE"}"#,
        )
        .unwrap();
        assert_eq!(response.routing_fee_msat, 526);
        assert_eq!(response.time_lock_delay, 682);
    }
}
