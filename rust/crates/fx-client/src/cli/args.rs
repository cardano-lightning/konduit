use std::time::Duration;

use crate::BaseCurrency;

// FIXME
// https://github.com/clap-rs/clap/issues/4279
// Clap's derive `Args` uses the struct name verbatim.
// There doesn't seem to be any attribute declaration to get around this.
// I think we still want to use derive since its fewer LoC.
// However we now do this weird dance of `pub struct MyServiceArgs` and `pub use
// args::MyServiceArgs as Args`.

#[derive(Debug, Clone, clap::Parser)]
pub struct FxArgs {
    #[arg(long, env = "FX_EVERY", value_parser = humantime::parse_duration, default_value = "2s")]
    #[cfg_attr(feature = "namespaced", arg(long("fx-every")))]
    pub every: Duration,

    #[arg(long, env = "FX_BASE_CURRENCY", value_enum, default_value = "eur")]
    #[cfg_attr(feature = "namespaced", arg(long("fx-base-currency")))]
    pub base_currency: BaseCurrency,

    #[arg(long, env = "FX_BINANCE", default_value_t = false)]
    #[cfg_attr(feature = "namespaced", arg(long("fx-binance")))]
    pub binance: bool,

    #[arg(long, env = "FX_COIN_GECKO_TOKEN")]
    #[cfg_attr(feature = "namespaced", arg(long("fx-coin-gecko-token")))]
    pub coin_gecko_token: Option<String>,

    #[arg(long, env = "FX_COIN_GECKO", default_value_t = false)]
    #[cfg_attr(feature = "namespaced", arg(long("fx-coin-gecko")))]
    pub coin_gecko_public: bool,

    #[arg(long, env = "FX_BITCOIN")]
    #[cfg_attr(feature = "namespaced", arg(long("fx-bitcoin")))]
    pub bitcoin: Option<f64>,

    #[arg(long, env = "FX_ADA")]
    #[cfg_attr(feature = "namespaced", arg(long("fx-ada")))]
    pub ada: Option<f64>,

    #[arg(long, env = "FX_KRAKEN", default_value_t = false)]
    #[cfg_attr(feature = "namespaced", arg(long("fx-kraken")))]
    pub kraken: bool,
}
