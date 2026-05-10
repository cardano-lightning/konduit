//! TODO

use konduit_data::Receipt;

pub struct Backing(());

pub struct Response {
    backing: Option<Backing>,
    receipt: Option<Receipt>,
}
