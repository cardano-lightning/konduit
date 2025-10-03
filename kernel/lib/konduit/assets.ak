use cardano/assets.{Value}
use extra/xassets.{only_ada}
use konduit/prelude.{Amount}

/// #### Amount in
/// 
/// A channel supports only ada at time of writing.
pub fn amount_in(value: Value) -> Amount {
  value |> assets.lovelace_of
}

/// #### Amount out
/// 
/// A channel supports only ada at time of writing.
pub fn amount_out(value: Value) -> Amount {
  only_ada(value)
}
