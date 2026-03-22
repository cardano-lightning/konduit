use async_trait::async_trait;

use crate::Api;

/// `Merchant` is an extension of `Api` predominantly geared towards testing.

#[async_trait]
pub trait Merchant {
    /// Proof of life
    async fn health(&self) -> crate::Result<String>;

    /// Manually inject an invoice into the system.
    async fn add_invoice(&self, amount_msat: Option<u64>) -> crate::Result<()>;
}

pub trait MerchantApi: Api + Merchant {}

impl<T> MerchantApi for T 
where 
    T: Api + Merchant 
{}
