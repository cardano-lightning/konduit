use cardano/assets.{Assets}
use cardano/value as value
use extra/xassets.{only_ada}
use konduit/prelude.{Amount}

/// #### Amount in
/// 
/// A channel supports only ada at time of writing.
pub fn amount_in(assets: Assets) -> Amount {
  assets |> value.from_assets |> value.lovelace_of
}

/// #### Amount out
/// 
/// A channel supports only ada at time of writing.
pub fn amount_out(assets: Assets) -> Amount {
  assets |> value.from_assets |> only_ada
}
