use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub index: u64,
    pub amount: u64,
    pub relative_timeout: u64,
    // TODO (@waalge) TBD whether these fields are relevant.
    // #[serde(with = "hex")]
    // pub lock: [u8; 32],
    // #[serde(with = "hex")]
    // pub payee: [u8; 33],
    // pub amount_msat: u64,
    // #[serde(with = "hex")]
    // pub payment_secret: [u8; 32],
    pub routing_fee: u64,
}

#[cfg(feature = "wasm")]
pub mod wasm {
    use cardano_sdk::wasm_proxy;
    use serde::{Deserialize, Serialize};
    use wasm_bindgen::prelude::*;

    wasm_proxy! {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        Quote
    }

    #[wasm_bindgen]
    impl Quote {
        #[wasm_bindgen(getter, js_name = "index")]
        pub fn index(&self) -> u64 {
            self.index
        }

        #[wasm_bindgen(getter, js_name = "amount")]
        pub fn amount(&self) -> u64 {
            self.amount
        }

        #[wasm_bindgen(getter, js_name = "relativeTimeout")]
        pub fn relative_timeout(&self) -> u64 {
            self.relative_timeout
        }

        #[wasm_bindgen(getter, js_name = "routingFee")]
        pub fn routing_fee(&self) -> u64 {
            self.routing_fee
        }
    }
}
