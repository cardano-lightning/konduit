use crate::models::Constants;

pub fn constants() -> std::io::Result<Constants> {
    let adaptor_key = std::env::var("KONDUIT_ADAPTOR_KEY").unwrap();
    let close_period = std::env::var("KONDUIT_CLOSE_PERIOD").unwrap();
    let constants = Constants {
        adaptor_key: <[u8; 32]>::try_from(hex::decode(adaptor_key).unwrap()).unwrap(),
        close_period: close_period.parse::<u64>().unwrap(),
    };
    Ok(constants)
}
