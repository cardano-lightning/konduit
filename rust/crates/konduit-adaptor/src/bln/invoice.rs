use std::time::Duration;

use bitcoin_hashes::Hash;
use lightning_invoice::{
    self, Currency, Description, Fallback, MinFinalCltvExpiryDelta, PrivateRoute, RawHrp,
    RawTaggedField, SignedRawBolt11Invoice, TaggedField,
};
use lightning_types::features::Bolt11InvoiceFeatures;

#[derive(Debug, Clone, thiserror::Error)]
pub enum InvoiceError {
    #[error("Parse Error")]
    Parse,
    #[error("Bad input")]
    BadInput,
    #[error("Cannot handle picosatoshi")]
    AmountPico,
    #[error("Amount Overflow")]
    AmountOverflow,
    #[error("Missing field {0}")]
    MissingField(String),
}

#[derive(Debug, Clone)]
pub struct Invoice {
    pub __raw: Option<SignedRawBolt11Invoice>,
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

impl TryFrom<&str> for Invoice {
    type Error = InvoiceError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Invoice::try_from(
            value
                .parse::<SignedRawBolt11Invoice>()
                .map_err(|_| InvoiceError::Parse)?,
        )
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
                __raw: Some(value),
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

#[derive(Default)]
struct TaggedFields {
    payment_hash: Option<[u8; 32]>,
    description: Option<Description>,
    payee_pub_key: Option<[u8; 33]>,
    description_hash: Option<[u8; 32]>,
    expiry_time: Option<Duration>,
    min_final_cltv_expiry_delta: Option<MinFinalCltvExpiryDelta>,
    fallback: Option<Fallback>,
    private_route: Option<PrivateRoute>,
    payment_secret: Option<[u8; 32]>,
    payment_metadata: Option<Vec<u8>>,
    features: Option<Bolt11InvoiceFeatures>,
}

impl From<Vec<RawTaggedField>> for TaggedFields {
    fn from(value: Vec<RawTaggedField>) -> Self {
        let mut tfs = TaggedFields::default();
        value.iter().for_each(|tagged_field| match tagged_field {
            lightning_invoice::RawTaggedField::KnownSemantics(tagged_field) => {
                match tagged_field.clone() {
                    TaggedField::PaymentHash(sha256) => {
                        tfs.payment_hash = Some(sha256.0.to_byte_array())
                    }
                    TaggedField::Description(description) => tfs.description = Some(description),
                    TaggedField::PayeePubKey(payee_pub_key) => {
                        tfs.payee_pub_key = Some(payee_pub_key.0.serialize())
                    }
                    TaggedField::DescriptionHash(sha256) => {
                        tfs.description_hash = Some(sha256.0.to_byte_array())
                    }
                    TaggedField::ExpiryTime(expiry_time) => {
                        tfs.expiry_time = Some(*expiry_time.as_duration())
                    }
                    TaggedField::MinFinalCltvExpiryDelta(min_final_cltv_expiry_delta) => {
                        tfs.min_final_cltv_expiry_delta = Some(min_final_cltv_expiry_delta)
                    }
                    TaggedField::Fallback(fallback) => tfs.fallback = Some(fallback),
                    TaggedField::PrivateRoute(private_route) => {
                        tfs.private_route = Some(private_route)
                    }
                    TaggedField::PaymentSecret(payment_secret) => {
                        tfs.payment_secret = Some(payment_secret.0)
                    }
                    TaggedField::PaymentMetadata(items) => tfs.payment_metadata = Some(items),
                    TaggedField::Features(features) => tfs.features = Some(features),
                }
            }
            lightning_invoice::RawTaggedField::UnknownSemantics(_fe32s) => {}
        });
        tfs
    }
}
