//! Generates the konduit-api CDDL spec.
//!
//! Usage:
//!   cargo run -p konduit-api --bin generate_spec --features cddl > spec/konduit.cddl
//!
//! To add a new type: derive `ToCddl` on it, then add one line to the list below.

fn main() {
    use konduit_api::{
        DepthBucket,
        auth::pop,
        common::channel::{SquashProposal, quote},
        endpoints::{
            channel::{pay::quoted, squash, sync},
            info, version,
        },
    };
    use konduit_cddl::ToCddl;

    let cddl = konduit_cddl::collect(&[
        // Auth
        pop::Error::cddl_definition,
        // Channel backing (from konduit-channel)
        DepthBucket::cddl_definition,
        // GET /channel/sync
        sync::BackingView::cddl_definition,
        sync::Response::cddl_definition,
        sync::Error::cddl_definition,
        // Common: squash proposal (returned in sync-response)
        SquashProposal::cddl_definition,
        // POST /channel/squash
        squash::Request::cddl_definition,
        squash::Response::cddl_definition,
        squash::Error::cddl_definition,
        // POST /channel/pay/quoted
        quoted::Request::cddl_definition,
        quoted::Response::cddl_definition,
        quoted::Error::cddl_definition,
        // Common quote types (shared by all /channel/quote/* endpoints)
        quote::Response::cddl_definition,
        quote::Error::cddl_definition,
        // GET /info
        info::Response::cddl_definition,
        info::TosInfo::cddl_definition,
        info::ChannelParameters::cddl_definition,
        info::TxHelp::cddl_definition,
        // GET /version
        version::Response::cddl_definition,
        version::FeatureInfo::cddl_definition,
        version::VcsHash::cddl_definition,
        version::SemVer::cddl_definition,
    ]);

    print!("{cddl}");
}
