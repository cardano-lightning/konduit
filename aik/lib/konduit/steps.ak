use aiken/crypto.{VerificationKey}
use cardano/assets.{Value}
use env
use konduit/assets.{amount_in, amount_out} as k_assets
use konduit/types as t
use konduit/wellformed.{
  verify_end, verify_expire, verify_m_receipt, verify_receipt, verify_unlock,
}

pub type Return<result> =
  fn(VerificationKey, Option<Int>, Option<Int>) -> result

pub fn do_add(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Opened(_) = stage_in
  expect stage_in == stage_out
  expect amount_in(value_in) < amount_out(value_out)
  return(add_vkey, None, None)
}

pub fn do_sub(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  squash: t.Squash,
  unlockeds: List<t.Unlocked>,
  return: Return<result>,
) -> result {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Opened(subbed_in) = stage_in
  expect t.Opened(subbed_out) = stage_out
  expect subbed_out - subbed_in == amount_in(value_in) - amount_out(value_out)
  let amount, upper_bound <-
    verify_receipt(tag, add_vkey, squash, unlockeds, _)
  expect subbed_out <= amount
  return(sub_vkey, None, upper_bound)
}

pub fn do_close(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, close_period, .. } = constants
  expect t.Opened(subbed) = stage_in
  expect t.Closed(subbed_, elapse_at) = stage_out
  expect subbed == subbed_
  expect amount_in(value_in) <= amount_out(value_out)
  return(add_vkey, None, Some(elapse_at - close_period))
}

pub fn do_respond(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  squash: t.Squash,
  mixs: List<t.Mix>,
  return: Return<result>,
) -> result {
  let t.Constants { tag, add_vkey, sub_vkey, .. } = constants
  expect t.Closed(subbed_in, _) = stage_in
  expect t.Responded(pends_amount, pends) = stage_out
  let subbed_out = subbed_in - amount_in(value_in) + amount_out(value_out)
  let amount, upper_bound <-
    verify_m_receipt(tag, add_vkey, squash, mixs, pends_amount, pends, _)
  expect amount >= subbed_out
  return(sub_vkey, None, upper_bound)
}

pub fn do_unlock(
  constants: t.Constants,
  value_in: Value,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  unpends: List<t.Unpend>,
  return: Return<result>,
) -> result {
  let t.Constants { sub_vkey, .. } = constants
  expect t.Responded(pends_amount_in, pends_in) = stage_in
  expect t.Responded(pends_amount_out, pends_out) = stage_out
  let subbed = amount_in(value_in) - amount_out(value_out)
  let amount, upper_bound <- verify_unlock(unpends, pends_in, pends_out)
  expect pends_amount_out == pends_amount_in - amount
  expect amount >= subbed
  return(sub_vkey, None, upper_bound)
}

pub fn do_expire(
  constants: t.Constants,
  stage_in: t.Stage,
  value_out: Value,
  stage_out: t.Stage,
  unpends: List<t.Unpend>,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(pends_amount_in, pends_in) = stage_in
  expect t.Responded(pends_amount_out, pends_out) = stage_out
  let amount, lower_bound <-
    verify_expire(unpends, pends_in, pends_out, _)
  expect pends_amount_out == pends_amount_in - amount
  expect amount_out(value_out) >= pends_amount_out + env.min_ada_buffer
  return(add_vkey, lower_bound, None)
}

pub fn do_end(
  constants: t.Constants,
  stage_in: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Responded(_, pends_in) = stage_in
  let lower_bound <-
    verify_end(pends_in, _)
  return(add_vkey, lower_bound, None)
}

pub fn do_elapse(
  constants: t.Constants,
  stage_in: t.Stage,
  return: Return<result>,
) -> result {
  let t.Constants { add_vkey, .. } = constants
  expect t.Closed(_, expire_at) = stage_in
  return(add_vkey, Some(expire_at), None)
}
