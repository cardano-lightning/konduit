use cardano_sdk::SigningKey;
use http_client_native::HttpClient;
use konduit_data::Tag;
use konduit_tx::ChannelUtxo;

use crate::{Adaptor, Cardano};

pub struct Channel<'a> {
    name: String,
    signing_key: &'a SigningKey,
    tag: Tag,
    adaptor: &'a Adaptor<HttpClient>,
}

pub struct Cache {
    l1: Option<ChannelUtxo>,
    l2: String,
}
