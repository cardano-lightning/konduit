use cardano/assets
use konduit/steps/add
use konduit/tests/fixture as f

// add.do returns the required vkey; it does not check the signature itself —
// main.handle collects vkeys and verifies signatories.

test test_add_ok() {
  let vkey =
    add.do(
      f.constants,
      assets.from_lovelace(100_000_000),
      f.opened(0, []),
      assets.from_lovelace(200_000_000),
      f.opened(0, []),
    )
  vkey == f.add_vkey
}

test test_add_equal_amount() fail {
  // amount_out must be strictly greater than amount_in
  expect _ =
    add.do(
      f.constants,
      assets.from_lovelace(100_000_000),
      f.opened(0, []),
      assets.from_lovelace(100_000_000),
      f.opened(0, []),
    )
}

test test_add_decreased_amount() fail {
  expect _ =
    add.do(
      f.constants,
      assets.from_lovelace(200_000_000),
      f.opened(0, []),
      assets.from_lovelace(100_000_000),
      f.opened(0, []),
    )
}

test test_add_stage_mismatch() fail {
  // stage_in must equal stage_out
  expect _ =
    add.do(
      f.constants,
      assets.from_lovelace(100_000_000),
      f.opened(0, []),
      assets.from_lovelace(200_000_000),
      f.opened(1, []),
    )
}

test test_add_wrong_stage() fail {
  // stage_in must be Opened
  expect _ =
    add.do(
      f.constants,
      assets.from_lovelace(100_000_000),
      f.closed(0, [], f.much_later),
      assets.from_lovelace(200_000_000),
      f.closed(0, [], f.much_later),
    )
}
