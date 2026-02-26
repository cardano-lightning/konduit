use std::fmt;

use base64::{Engine, engine::general_purpose};

#[derive(Debug, Clone)]
pub struct TlsCertificate(Vec<u8>);

impl std::str::FromStr for TlsCertificate {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        general_purpose::STANDARD
            .decode(s)
            .map_err(|err| err.to_string())
            .map(TlsCertificate)
    }
}

impl AsRef<[u8]> for TlsCertificate {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl fmt::Display for TlsCertificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
            general_purpose::STANDARD.encode(&self.0)
        )
    }
}
