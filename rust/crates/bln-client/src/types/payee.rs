#[derive(Debug, Clone)]
pub struct Payee(pub [u8; 33]);

impl std::str::FromStr for Payee {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let vec = hex::decode(s).map_err(|err| err.to_string())?;
        let arr = <[u8; 33]>::try_from(vec).map_err(|_| "Wrong length".to_string())?;
        Ok(Payee(arr))
    }
}

impl AsRef<[u8; 33]> for Payee {
    fn as_ref(&self) -> &[u8; 33] {
        &self.0
    }
}
