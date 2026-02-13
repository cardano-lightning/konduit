use bitcoin::hashes::Hash;
use lightning_invoice::{
    Description, Fallback, MinFinalCltvExpiryDelta, PrivateRoute, RawTaggedField, TaggedField,
};
use lightning_types::features::Bolt11InvoiceFeatures;
use std::time::Duration;

#[derive(Default)]
pub(crate) struct TaggedFields {
    pub(crate) payment_hash: Option<[u8; 32]>,
    pub(crate) description: Option<Description>,
    pub(crate) payee_pub_key: Option<[u8; 33]>,
    pub(crate) description_hash: Option<[u8; 32]>,
    pub(crate) expiry_time: Option<Duration>,
    pub(crate) min_final_cltv_expiry_delta: Option<MinFinalCltvExpiryDelta>,
    pub(crate) fallback: Option<Fallback>,
    pub(crate) private_route: Option<PrivateRoute>,
    pub(crate) payment_secret: Option<[u8; 32]>,
    pub(crate) payment_metadata: Option<Vec<u8>>,
    pub(crate) features: Option<Bolt11InvoiceFeatures>,
}

impl From<Vec<RawTaggedField>> for TaggedFields {
    fn from(value: Vec<RawTaggedField>) -> Self {
        let mut tfs = TaggedFields::default();
        value.iter().for_each(|tagged_field| match tagged_field {
            RawTaggedField::KnownSemantics(tagged_field) => {
                use TaggedField::*;
                match tagged_field {
                    PaymentHash(sha256) => tfs.payment_hash = Some(sha256.0.to_byte_array()),
                    Description(description) => tfs.description = Some(description.clone()),
                    PayeePubKey(payee_pub_key) => {
                        tfs.payee_pub_key = Some(payee_pub_key.0.serialize())
                    }
                    DescriptionHash(sha256) => {
                        tfs.description_hash = Some(sha256.0.to_byte_array())
                    }
                    ExpiryTime(expiry_time) => tfs.expiry_time = Some(*expiry_time.as_duration()),
                    TaggedField::MinFinalCltvExpiryDelta(min_final_cltv_expiry_delta) => {
                        tfs.min_final_cltv_expiry_delta = Some(min_final_cltv_expiry_delta.clone())
                    }
                    Fallback(fallback) => tfs.fallback = Some(fallback.clone()),
                    PrivateRoute(private_route) => tfs.private_route = Some(private_route.clone()),
                    PaymentSecret(payment_secret) => tfs.payment_secret = Some(payment_secret.0),
                    PaymentMetadata(items) => tfs.payment_metadata = Some(items.to_vec()),
                    Features(features) => tfs.features = Some(features.clone()),
                }
            }
            RawTaggedField::UnknownSemantics(_fe32s) => {}
        });
        tfs
    }
}
