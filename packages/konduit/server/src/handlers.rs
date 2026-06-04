use konduit_wire::info;

use crate::State;

pub async fn info(state: &State) -> info::Response {
    state.info().as_ref().clone()
}
