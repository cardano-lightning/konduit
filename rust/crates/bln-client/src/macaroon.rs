#[derive(Debug, Clone)]
pub struct Macaroon(Vec<u8>);

impl std::str::FromStr for Macaroon {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        hex::decode(s).map_err(|err| err.to_string()).map(Macaroon)
    }
}

impl AsRef<[u8]> for Macaroon {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
