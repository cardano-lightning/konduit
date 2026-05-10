//! Generates the konduit-api OpenAPI spec.
//!
//! Usage:
//!   cargo run -p konduit-api --bin generate_openapi --features openapi > spec/konduit.json
//!
//! The output is a JSON document conforming to OpenAPI 3.1.
//! Path annotations live here (not in the library) so the library stays framework-agnostic.

use konduit_api::{
    auth::pop,
    common::channel::{SquashProposal, quote},
    endpoints::{
        channel::{pay::quoted, squash, sync},
        info, version,
    },
};
use utoipa::OpenApi;

// ---------------------------------------------------------------------------
// Path stubs — annotated functions that document each endpoint.
// No implementation; the server wires up the actual handlers.
// ---------------------------------------------------------------------------

/// Returns version and feature compatibility information.
#[utoipa::path(
    get, path = "/version",
    responses(
        (status = 200, description = "Server version", body = version::Response),
    )
)]
fn get_version() {}

/// Returns server parameters the client needs to open a channel.
#[utoipa::path(
    get, path = "/info",
    responses(
        (status = 200, description = "Server info", body = info::Response),
    )
)]
fn get_info() {}

/// Returns the server's current view of the channel state.
#[utoipa::path(
    get, path = "/channel/sync",
    security(("Konduit-Keytag" = [], "Konduit-Signature" = [])),
    responses(
        (status = 200, description = "Channel state", body = sync::Response),
        (status = 400, description = "Auth or rate-limit error", body = sync::Error),
        (status = 401, description = "Bad auth", body = sync::Error),
        (status = 404, description = "No backing", body = sync::Error),
        (status = 429, description = "Rate limited", body = sync::Error),
    )
)]
fn get_channel_sync() {}

/// Submits a signed squash to advance the channel's squash state.
#[utoipa::path(
    post, path = "/channel/squash",
    security(("Konduit-Keytag" = [], "Konduit-Signature" = [])),
    request_body = squash::Request,
    responses(
        (status = 200, description = "Squash accepted", body = squash::Response),
        (status = 401, description = "Bad auth", body = squash::Error),
        (status = 404, description = "No backing", body = squash::Error),
        (status = 409, description = "Stale: re-sync required", body = squash::Error),
        (status = 422, description = "Invalid squash", body = squash::Error),
        (status = 429, description = "Rate limited", body = squash::Error),
    )
)]
fn post_channel_squash() {}

/// Submits a signed cheque to make a quoted payment.
#[utoipa::path(
    post, path = "/channel/pay/quoted",
    security(("Konduit-Keytag" = [], "Konduit-Signature" = [])),
    request_body = quoted::Request,
    responses(
        (status = 200, description = "Payment result", body = quoted::Response),
        (status = 400, description = "Bad cheque or timeout", body = quoted::Error),
        (status = 401, description = "Bad auth", body = quoted::Error),
        (status = 402, description = "Insufficient funds", body = quoted::Error),
        (status = 404, description = "No backing", body = quoted::Error),
        (status = 422, description = "Lock mismatch", body = quoted::Error),
        (status = 428, description = "Squash required", body = quoted::Error),
        (status = 429, description = "Rate limited or capacity exceeded", body = quoted::Error),
        (status = 502, description = "Routing failed", body = quoted::Error),
    )
)]
fn post_channel_pay_quoted() {}

/// Quotes a payment for a BOLT-11 invoice.
#[utoipa::path(
    post, path = "/channel/quote/bolt11",
    security(("Konduit-Keytag" = [], "Konduit-Signature" = [])),
    responses(
        (status = 200, description = "Cheque proposal", body = quote::Response),
        (status = 402, description = "Insufficient funds", body = quote::Error),
        (status = 404, description = "No backing or route not found", body = quote::Error),
        (status = 413, description = "Payload too large", body = quote::Error),
        (status = 428, description = "Squash required", body = quote::Error),
        (status = 429, description = "Rate limited or capacity exceeded", body = quote::Error),
    )
)]
fn post_channel_quote_bolt11() {}

/// Quotes a payment using BLN template (no invoice required).
#[utoipa::path(
    post, path = "/channel/quote/bln-template",
    security(("Konduit-Keytag" = [], "Konduit-Signature" = [])),
    responses(
        (status = 200, description = "Cheque proposal", body = quote::Response),
        (status = 402, description = "Insufficient funds", body = quote::Error),
        (status = 404, description = "No backing or route not found", body = quote::Error),
        (status = 428, description = "Squash required", body = quote::Error),
        (status = 429, description = "Rate limited or capacity exceeded", body = quote::Error),
    )
)]
fn post_channel_quote_bln_template() {}

/// Quotes a payment using CL template (no invoice required).
#[utoipa::path(
    post, path = "/channel/quote/cl-template",
    security(("Konduit-Keytag" = [], "Konduit-Signature" = [])),
    responses(
        (status = 200, description = "Cheque proposal", body = quote::Response),
        (status = 402, description = "Insufficient funds", body = quote::Error),
        (status = 404, description = "No backing or route not found", body = quote::Error),
        (status = 428, description = "Squash required", body = quote::Error),
        (status = 429, description = "Rate limited or capacity exceeded", body = quote::Error),
    )
)]
fn post_channel_quote_cl_template() {}

// ---------------------------------------------------------------------------
// API doc assembly
// ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Konduit API",
        description = "Konduit L2 payment channel protocol",
    ),
    paths(
        get_version,
        get_info,
        get_channel_sync,
        post_channel_squash,
        post_channel_pay_quoted,
        post_channel_quote_bolt11,
        post_channel_quote_bln_template,
        post_channel_quote_cl_template,
    ),
    components(
        schemas(
            // Auth
            pop::Error,
            // Sync
            sync::BackingView,
            sync::Response,
            sync::Error,
            // Squash proposal (in sync-response)
            SquashProposal,
            // Squash
            squash::Request,
            squash::Response,
            squash::Error,
            // Pay/quoted
            quoted::Request,
            quoted::Response,
            quoted::Error,
            // Common quote
            quote::Response,
            quote::Error,
            // Info
            info::Response,
            info::TosInfo,
            info::ChannelParameters,
            info::TxHelp,
            // Version
            version::Response,
            version::FeatureInfo,
            version::VcsHash,
            version::SemVer,
        )
    ),
    security(
        ("Konduit-Keytag" = []),
        ("Konduit-Signature" = []),
    )
)]
struct ApiDoc;

fn main() {
    print!(
        "{}",
        ApiDoc::openapi()
            .to_pretty_json()
            .expect("serialize openapi")
    );
}
