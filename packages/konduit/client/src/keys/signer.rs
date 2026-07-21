//! A black-box signer: takes a message, returns a signature over it.
//! Deliberately says nothing about what's behind it — a local key, an HSM,
//! a remote KMS call — only that it can sign.
use cardano_sdk::{Signature, SigningKey, VerificationKey};

pub trait Signer: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn sign(
        &self,
        message: &[u8],
    ) -> impl std::future::Future<Output = Result<Signature, Self::Error>> + Send;

    fn verification_key(&self) -> VerificationKey;
}

impl Signer for SigningKey {
    type Error = std::convert::Infallible;

    async fn sign(&self, message: &[u8]) -> Result<Signature, Self::Error> {
        Ok(self.sign(message)) // adjust to SigningKey's real cardano_sdk-based sign call
    }

    fn verification_key(&self) -> VerificationKey {
        self.to_verification_key() // adjust to SigningKey's real method name
    }
}
