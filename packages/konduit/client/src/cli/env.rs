//! env.rs — resolve config values that reference environment variables.
//!
//! A config value of the literal form `env:VAR_NAME` is resolved by
//! reading `VAR_NAME` from the process environment (expected to already
//! be populated, e.g. by `dotenvy::dotenv()` at startup) rather than
//! being used as-is. This lets `konduit.toml` say
//! `wallet = "env:KONDUIT_WALLET_KEY"` to keep secret material out of
//! the file entirely, with the var name itself chosen by the user
//! rather than fixed by konduit.

const ENV_PREFIX: &str = "env:";

/// Resolve `value`: if it's of the form `env:VAR_NAME`, look up
/// `VAR_NAME` in the environment; otherwise return `value` unchanged.
pub fn resolve(value: &str) -> anyhow::Result<String> {
    match value.strip_prefix(ENV_PREFIX) {
        Some(var_name) => std::env::var(var_name).map_err(|_| {
            anyhow::anyhow!("`{value}` references unset environment variable `{var_name}`")
        }),
        None => Ok(value.to_string()),
    }
}
