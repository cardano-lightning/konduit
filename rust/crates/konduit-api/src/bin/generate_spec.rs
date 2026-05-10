//! Generates the konduit-api CDDL spec.
//!
//! Usage:
//!   cargo run -p konduit-api --bin generate_spec --features cddl > spec/konduit.cddl
//!
//! To add a new type: derive `ToCddl` on it, then add one line to the list below.

fn main() {
    use konduit_api::{
        auth::pop,
        channel::DepthBucket,
        common::channel::quote,
        endpoints::channel::{pay::quoted, squash, sync},
    };
    use konduit_cddl::ToCddl;

    let cddl = konduit_cddl::collect(&[
        // Auth
        pop::Error::cddl_definition,
        // Channel backing
        DepthBucket::cddl_definition,
        // GET /channel/sync
        sync::BackingView::cddl_definition,
        sync::Response::cddl_definition,
        sync::Error::cddl_definition,
        // POST /channel/squash
        squash::Request::cddl_definition,
        squash::Response::cddl_definition,
        squash::Error::cddl_definition,
        // POST /channel/pay/quoted
        quoted::Request::cddl_definition,
        quoted::Response::cddl_definition,
        quoted::Error::cddl_definition,
        // Common quote error
        quote::Error::cddl_definition,
    ]);

    print!("{cddl}");
}
