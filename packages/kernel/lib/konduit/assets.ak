use aiken/collection/dict.{to_pairs}
use aiken/primitive/bytearray
use cardano/assets.{Value, to_dict}
use env
use konduit/prelude.{Amount}
use konduit/types.{Ada, AssetId, Native}

pub fn amount_in(value: Value, asset: AssetId) -> Amount {
  amount(value, asset)
}

pub fn amount_out(value: Value, asset: AssetId) -> Amount {
  amount(value, asset)
}

fn lovelace(value: Value) -> Int {
  expect [Pair(ada_policy, ada_assets), ..] = value
    |> to_dict
    |> to_pairs
  expect ada_policy == #""
  expect [Pair(ada_name, lovelace)] = ada_assets |> to_pairs
  expect ada_name == #""
  lovelace
}

pub fn preserve_lovelace(
  value_in: Value,
  value_out: Value,
  asset: AssetId,
) -> Bool {
  when asset is {
    Ada -> True
    Native(_, _) -> lovelace(value_in) <= lovelace(value_out)
  }
}

fn amount(value: Value, asset: AssetId) -> Amount {
  expect [Pair(ada_policy, ada_assets), ..native_assets] = value
    |> to_dict
    |> to_pairs
  expect ada_policy == #""
  expect [Pair(ada_name, lovelace)] = ada_assets |> to_pairs
  expect ada_name == #""
  expect lovelace >= env.min_ada_buffer
  when asset is {
    Ada -> {
      expect native_assets == []
      lovelace - env.min_ada_buffer
    }
    Native(policy, name) -> {
      expect bytearray.length(policy) == 28
      expect bytearray.length(name) <= 32
      when native_assets is {
        [] -> 0
        [Pair(actual_policy, assets)] -> {
          expect actual_policy == policy
          expect [Pair(actual_name, quantity)] = assets |> to_pairs
          expect actual_name == name
          expect quantity > 0
          quantity
        }
        _ -> fail
      }
    }
  }
}

const test_policy = #"11111111111111111111111111111111111111111111111111111111"

const test_name = #"544f4b454e"

fn native_value(quantity: Int) -> Value {
  assets.from_lovelace(env.min_ada_buffer)
    |> assets.add(test_policy, test_name, quantity)
}

test native_capacity_and_subtraction() {
  amount_out(native_value(25_000_000), Native(test_policy, test_name)) == 25_000_000 && amount_out(
    native_value(20_000_000),
    Native(test_policy, test_name),
  ) == 20_000_000
}

test native_zero_capacity() {
  amount_out(
    assets.from_lovelace(env.min_ada_buffer),
    Native(test_policy, test_name),
  ) == 0
}

test ada_capacity_excludes_reserve() {
  amount_out(assets.from_lovelace(env.min_ada_buffer + 25), Ada) == 25
}

test wrong_policy_fails() fail {
  amount_out(
    native_value(1),
    Native(
      #"22222222222222222222222222222222222222222222222222222222",
      test_name,
    ),
  ) == 0
}

test wrong_name_fails() fail {
  amount_out(native_value(1), Native(test_policy, #"00")) == 0
}

test second_token_fails() fail {
  let value = native_value(1) |> assets.add(test_policy, #"00", 1)
  amount_out(value, Native(test_policy, test_name)) == 0
}

test invalid_policy_length_fails() fail {
  amount_out(assets.from_lovelace(env.min_ada_buffer), Native(#"00", test_name)) == 0
}

test invalid_name_length_fails() fail {
  amount_out(
    assets.from_lovelace(env.min_ada_buffer),
    Native(
      test_policy,
      #"000000000000000000000000000000000000000000000000000000000000000000",
    ),
  ) == 0
}

test insufficient_reserve_fails() fail {
  amount_out(
    assets.from_lovelace(env.min_ada_buffer - 1),
    Native(test_policy, test_name),
  ) == 0
}

test extra_asset_in_ada_fails() fail {
  amount_out(native_value(1), Ada) == 0
}

test native_extra_lovelace_is_preserved() {
  let asset = Native(test_policy, test_name)
  let value = assets.from_lovelace(env.min_ada_buffer + 1)
    |> assets.add(test_policy, test_name, 1)
  preserve_lovelace(value, value, asset)
}

test native_lovelace_can_be_topped_up() {
  let asset = Native(test_policy, test_name)
  let value_in = assets.from_lovelace(env.min_ada_buffer)
    |> assets.add(test_policy, test_name, 1)
  let value_out = assets.from_lovelace(env.min_ada_buffer + 1)
    |> assets.add(test_policy, test_name, 1)
  preserve_lovelace(value_in, value_out, asset)
}

test native_extra_lovelace_cannot_be_siphoned() fail {
  let asset = Native(test_policy, test_name)
  let value_in = assets.from_lovelace(env.min_ada_buffer + 1)
    |> assets.add(test_policy, test_name, 1)
  let value_out = assets.from_lovelace(env.min_ada_buffer)
    |> assets.add(test_policy, test_name, 1)
  preserve_lovelace(value_in, value_out, asset)
}
