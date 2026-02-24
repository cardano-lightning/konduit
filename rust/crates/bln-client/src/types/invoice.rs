use crate::types::TaggedFields;
use error::InvoiceError;
use lightning_invoice::{
    Currency, Description, Fallback, MinFinalCltvExpiryDelta, PrivateRoute, RawHrp,
    SignedRawBolt11Invoice,
};
use lightning_types::features::Bolt11InvoiceFeatures;
use std::{fmt::Display, str::FromStr, time::Duration};

pub mod error;

#[derive(Debug, Clone)]
pub struct Invoice {
    pub __raw: SignedRawBolt11Invoice,
    pub invoice_hash: [u8; 32],
    pub currency: Currency,
    pub amount_msat: u64,
    pub payee_compressed: [u8; 33],
    pub payment_hash: [u8; 32],
    pub payment_secret: [u8; 32],
    pub features: Bolt11InvoiceFeatures,
    pub description: Option<Description>,
    pub description_hash: Option<[u8; 32]>,
    pub expiry_time: Option<Duration>,
    pub min_final_cltv_expiry_delta: Option<MinFinalCltvExpiryDelta>,
    pub fallback: Option<Fallback>,
    pub private_route: Option<PrivateRoute>,
    pub payment_metadata: Option<Vec<u8>>,
}

impl Display for Invoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.__raw)
    }
}

impl FromStr for Invoice {
    type Err = InvoiceError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Invoice::try_from(
            s.parse::<SignedRawBolt11Invoice>()
                .map_err(|_| InvoiceError::Parse)?,
        )
    }
}

// FIXME :: This should be removed
impl TryFrom<&String> for Invoice {
    type Error = InvoiceError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Invoice::try_from(
            value
                .parse::<SignedRawBolt11Invoice>()
                .map_err(|_| InvoiceError::Parse)?,
        )
    }
}

impl From<Invoice> for String {
    fn from(value: Invoice) -> Self {
        value.__raw.to_string()
    }
}

impl TryFrom<SignedRawBolt11Invoice> for Invoice {
    type Error = InvoiceError;

    fn try_from(value: SignedRawBolt11Invoice) -> Result<Self, Self::Error> {
        if value.check_signature() {
            let payee_compressed = value.recover_payee_pub_key().unwrap().0.serialize();
            let (invoice, hash, _sig) = value.clone().into_parts();
            let amount_msat = amount_msat(&invoice.hrp)?;
            let currency = invoice.hrp.currency;
            let tagged_fields = TaggedFields::from(invoice.data.tagged_fields);
            Ok(Self {
                __raw: value,
                invoice_hash: hash,
                amount_msat,
                currency,
                payee_compressed,
                payment_hash: tagged_fields
                    .payment_hash
                    .ok_or(InvoiceError::MissingField("payment_hash".to_string()))?,
                payment_secret: tagged_fields
                    .payment_secret
                    .ok_or(InvoiceError::MissingField("payment_secret".to_string()))?,
                features: tagged_fields
                    .features
                    .ok_or(InvoiceError::MissingField("features".to_string()))?,
                description: tagged_fields.description,
                description_hash: tagged_fields.description_hash,
                expiry_time: tagged_fields.expiry_time,
                min_final_cltv_expiry_delta: tagged_fields.min_final_cltv_expiry_delta,
                fallback: tagged_fields.fallback,
                private_route: tagged_fields.private_route,
                payment_metadata: tagged_fields.payment_metadata,
            })
        } else {
            Err(InvoiceError::BadInput)
        }
    }
}

fn amount_msat(hrp: &RawHrp) -> Result<u64, InvoiceError> {
    let RawHrp {
        raw_amount,
        si_prefix,
        ..
    } = hrp;
    let Some(raw_amount) = raw_amount else {
        return Err(InvoiceError::MissingField("raw_amount".to_owned()));
    };
    let Some(si_prefix) = si_prefix else {
        return Err(InvoiceError::MissingField("si_prefix".to_owned()));
    };
    let m = si_prefix.multiplier();
    if m == 1 {
        if raw_amount % 10 != 0 {
            Err(InvoiceError::AmountPico)
        } else {
            Ok(raw_amount / 10)
        }
    } else {
        raw_amount
            .checked_mul(m / 10)
            .ok_or(InvoiceError::AmountOverflow)
    }
}
