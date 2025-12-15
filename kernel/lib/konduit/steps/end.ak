use aiken/collection/list
use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use konduit/prelude.{Timebound}
use konduit/types as t

pub fn do(
  lower_bound: Timebound,
  constants: t.Constants,
  stage_in: t.Stage,
) -> VerificationKey {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(_, pends_in) = stage_in
  expect
    pends_in
      |> list.all(
          fn((_, timeout, _)) {
            expect Finite(lower_bound) = lower_bound
            timeout <= lower_bound
          },
        )
  add_vkey
}
