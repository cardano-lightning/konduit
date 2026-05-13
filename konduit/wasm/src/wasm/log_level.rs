use crate::wasm;
use anyhow::anyhow;
use wasm_bindgen::prelude::*;

/// A log level to configure the logger.
#[wasm_bindgen]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// To be called once in the application life-cycle to make logs from Rust/Wasm displayed in the
/// browser console, and to install a hook on Rust internal panics in order to make them bubble as
/// plain JavaScript errors.
#[wasm_bindgen(js_name = "enableLogsAndPanicHook")]
pub fn enable_logs_and_panic_hook(level: LogLevel) -> wasm::Result<()> {
    let log_level = match level {
        LogLevel::Trace => log::Level::Trace,
        LogLevel::Debug => log::Level::Debug,
        LogLevel::Info => log::Level::Info,
        LogLevel::Warn => log::Level::Warn,
        LogLevel::Error => log::Level::Error,
    };
    console_log::init_with_level(log_level).map_err(|_| anyhow!("console_log init failed"))?;
    console_error_panic_hook::set_once();
    Ok(())
}
