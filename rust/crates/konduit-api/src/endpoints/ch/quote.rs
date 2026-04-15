use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub enum QuoteBody {
    Simple(SimpleQuote),
    Bolt11(#[serde_as(as = "DisplayFromStr")] Invoice),
}

impl QuoteBody {
    pub fn amount_msat(&self) -> u64 {
        match self {
            QuoteBody::Simple(simple_quote) => simple_quote.amount_msat,
            QuoteBody::Bolt11(invoice) => invoice.amount_msat,
        }
    }

    pub fn payee(&self) -> [u8; 33] {
        match self {
            QuoteBody::Simple(simple_quote) => simple_quote.payee,
            QuoteBody::Bolt11(invoice) => invoice.payee_compressed.serialize(),
        }
    }

    pub fn route_hints(self) -> Vec<RouteHint> {
        match self {
            QuoteBody::Simple(simple_quote) => simple_quote.route_hints,
            QuoteBody::Bolt11(invoice) => invoice
                .private_route
                .into_iter()
                .map(|pr| RouteHint::from(pr.into_inner()))
                .collect::<Vec<_>>(),
        }
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize, Clone, Encode, Decode)]
pub struct SimpleQuote {
    pub amount_msat: u64,
    #[serde_as(as = "serde_with::hex::Hex")]
    pub payee: [u8; 33],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub route_hints: Vec<RouteHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Request {
    #[n(0)] Simple( #[n(0)] SimpleQuote),
    #[n(1)] Bolt11(#[serde_as(as = "DisplayFromStr")] Invoice),
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct Response {
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub enum Error {
}
