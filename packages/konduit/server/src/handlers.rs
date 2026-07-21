use cobbl3::{self, Body};
use konduit_wire::{info, limit::LimitError, reg};

use crate::{Channel, State, channel, db, time::now};

/// FIXME :: Move to config

pub const BUCKET_REGISTER: u64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Db :: {0}")]
    Db(#[from] db::Error),
}

type Handler<T, E> = Result<Result<T, E>, Error>;

trait HandlerExt<T, E> {
    fn reject(e: E) -> Self;
    fn accept(v: T) -> Self;
}

impl<T, E> HandlerExt<T, E> for Handler<T, E> {
    fn reject(e: E) -> Self {
        Ok(Err(e))
    }
    fn accept(v: T) -> Self {
        Ok(Ok(v))
    }
}

pub fn info(state: &State) -> info::Response {
    state.info().as_ref().clone()
}

pub fn token_keytag(token_body: &reg::cobbl3::TokenBody) -> Vec<u8> {
    [&token_body.key, token_body.tag.as_slice()].concat()
}

pub fn reg(
    body: reg::cobbl3::Body,
    state: &State,
) -> Handler<reg::cobbl3::Response, reg::cobbl3::Error> {
    let keytag = token_keytag(&body.token.body);

    // DB inspection cheaper than verify. Do first.
    let Some(channel) = state.db().get(&keytag)? else {
        return Handler::reject(reg::cobbl3::Error::Common(reg::CommonError::NoChannel));
    };

    if channel.bucket().available(now()) < BUCKET_REGISTER {
        return Handler::reject(reg::cobbl3::Error::Common(reg::CommonError::Limit(
            LimitError::Reached,
        )));
    }

    if !cryptoxide::ed25519::verify(
        &body.token.body.tbs_bytes(),
        &body.token.body.key,
        &body.token.signature,
    ) {
        return Handler::reject(reg::cobbl3::Error::Cobbl3(cobbl3::Error::ClientSignature));
    }

    let Ok(_) = state
        .db()
        .update(&keytag, channel::consume(BUCKET_REGISTER))
    else {
        return Handler::reject(reg::cobbl3::Error::Common(reg::CommonError::NoChannel));
    };

    Handler::accept(reg::cobbl3::Response(
        state.cobbl3_key().sign(&body.token.body),
    ))
}
