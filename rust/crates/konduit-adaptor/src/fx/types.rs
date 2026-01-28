use chrono::Utc;
use serde::Serialize;
use std::fmt;

use actix_web::{HttpRequest, HttpResponse, Responder, body::BoxBody};

#[derive(Debug, Clone, Serialize)]
pub struct Fx {
    pub created_at: i64,
    pub base: BaseCurrency,
    pub ada: f64,
    pub bitcoin: f64,
}

impl Responder for Fx {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        // Automatically returns 200 OK with application/json
        HttpResponse::Ok().json(self)
    }
}

impl Fx {
    pub fn new(base: BaseCurrency, ada: f64, bitcoin: f64) -> Self {
        Fx {
            created_at: Utc::now().timestamp(),
            base,
            ada,
            bitcoin,
        }
    }

    pub fn msat_to_lovelace(&self, amount: u64) -> u64 {
        (amount as f64 * self.bitcoin / (self.ada * 100_000.0)) as u64
    }

    pub fn lovelace_to_msat(&self, amount: u64) -> u64 {
        ((amount as f64 * self.ada * 100_000.0) / self.bitcoin) as u64
    }
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum BaseCurrency {
    Eur,
}

impl fmt::Display for BaseCurrency {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Eur => write!(f, "eur"),
        }
    }
}
