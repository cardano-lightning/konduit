use serde::Deserialize;

/// LND REST proxy wraps every object in a stream with a "result" field.
#[derive(Deserialize)]
pub struct StreamWrapper<T> {
    pub result: T,
}
