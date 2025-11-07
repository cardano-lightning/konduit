use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[wasm_bindgen]
pub fn enable_logs(level: LogLevel) {
    let log_level = match level {
        LogLevel::Trace => log::Level::Trace,
        LogLevel::Debug => log::Level::Debug,
        LogLevel::Info => log::Level::Info,
        LogLevel::Warn => log::Level::Warn,
        LogLevel::Error => log::Level::Error,
    };
    console_log::init_with_level(log_level).expect("console_log init failed");
    console_error_panic_hook::set_once();
}
