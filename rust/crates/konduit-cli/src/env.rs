use std::collections::HashMap;

/// Used for variables that are roughly constant per "instance"
/// These include signing keys and cardano connection

const PREFIX: &str = "KONDUIT_";

pub fn get_env() -> HashMap<String, String> {
    let mut env = HashMap::new();
    for (key, value) in std::env::vars() {
        if let Some(key) = key.strip_prefix(PREFIX) {
            env.insert(key.to_lowercase(), value);
        };
    }
    env
}
