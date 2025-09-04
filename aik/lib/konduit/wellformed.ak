use aiken/builtin.{append_bytearray, verify_ed25519_signature, sha2_256}
use aiken/cbor
use aiken/crypto.{
  Signature, VerificationKey, 
}
use extra/xlist.{drop_hard, foldl3}
use konduit/prelude.{Amount, Index, Timestamp}
use konduit/types as t

pub fn mk_verify_signature(key: VerificationKey, tag: t.Tag) {
  fn(body: Data, signature: Signature) {
    verify_ed25519_signature(key, append_bytearray(tag, cbor.serialise(body)), signature)
  }
}

pub fn verify_unlocked(
  upper_bound: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  unlocked: t.Unlocked,
) {
  let (body, signature, preimage) = unlocked
  let (_, _, timeout, image) = body
  and {
    mk_verify_signature(key, tag)(body, signature)?,
    (upper_bound < timeout)?,
    (sha2_256(preimage) == image)?,
  }
}

pub fn verify_maybe_squash(
  verify_signature: fn(Data, Signature) -> Bool,
  m_squash: Option<t.Squash>,
  return: fn(Amount, Index, t.Exclude) -> return,
) {
  when m_squash is {
    Some((body, signature)) -> {
      expect verify_signature(body, signature)
      let (amount, index, exclude) = body
      return(amount, index, exclude)
    }
    None -> return(0, 0, [])
  }
}

pub fn verify_receipt(
  upper_bound: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  receipt: t.Receipt,
) {
  let verify_signature = mk_verify_signature(key, tag)
  let (m_squash, unlockeds) = receipt
  let amount, index, exclude <- verify_maybe_squash(verify_signature, m_squash)
  let with = mk_with_unlocked(upper_bound, verify_signature)
  let amount, _index, _exclude <-
    foldl3(unlockeds, amount, index, exclude, with, _)
  amount
}

pub fn mk_with_unlocked(
  upper_bound: Timestamp,
  verify_signature: fn(Data, Signature) -> Bool,
) {
  fn(
    unlocked: t.Unlocked,
    amount: Amount,
    index: Index,
    exclude: t.Exclude,
    return: fn(Amount, Index, t.Exclude) -> Amount,
  ) {
    let (body, signature, secret) = unlocked
    expect verify_signature(body, signature)
    let (new_index, new_amount, timeout, image) = body
    expect upper_bound < timeout
    expect sha2_256(secret) == image
    if index < new_index {
      return(amount + new_amount, new_index, exclude)
    } else {
      return(amount + new_amount, index, drop_hard(exclude, new_index))
    }
  }
}
// pub fn verify_m_receipt(
//   upper_bound: Timestamp,
//   key: VerificationKey,
//   tag: t.Tag,
//   m_receipt: t.MReceipt,
//   pends_amount: Amount,
//   pend_cheques: List<t.PendCheque>,
// ) {
//   let (m_squash, mixs) = m_receipt
//   when m_squash is {
//     Some((squash_body, signature)) -> {
//       expect verify_signature(key, tag, squash_body, signature)
//       let (tot, index, exclude) = squash_body
//       fold_mix(
//         upper_bound,
//         key,
//         tag,
//         exclude,
//         tot,
//         index,
//         mixs,
//         pends_amount,
//         pend_cheques,
//       )
//     }
//     None ->
//       fold_mix(
//         upper_bound,
//         key,
//         tag,
//         [],
//         0,
//         0,
//         mixs,
//         pends_amount,
//         pend_cheques,
//       )
//   }
// }
// 
// pub fn fold_mix(
//   upper_bound: Timestamp,
//   key: VerificationKey,
//   tag: t.Tag,
//   exclude: List<Index>,
//   tot: Amount,
//   index_bound: Index,
//   mixs: List<t.Mix>,
//   pends_amount: Amount,
//   pend_cheques: List<t.PendCheque>,
// ) {
//   when mixs is {
//     [] -> {
//       expect pends_amount == 0
//       expect pend_cheques == []
//       tot
//     }
//     [fst, ..mixs] ->
//       when fst is {
//         t.MUnlocked(body, signature, preimage) -> {
//           let (index, amount, timeout, image) = body
//           expect verify_signature(key, tag, body, signature)
//           expect upper_bound < timeout
//           expect sha2_256(preimage) == image
//           if index_bound < index {
//             fold_mix(
//               upper_bound,
//               key,
//               tag,
//               exclude,
//               tot + amount,
//               index,
//               mixs,
//               pends_amount,
//               pend_cheques,
//             )
//           } else {
//             fold_mix(
//               upper_bound,
//               key,
//               tag,
//               drop_hard(exclude, index_bound),
//               tot + amount,
//               index_bound,
//               mixs,
//               pends_amount,
//               pend_cheques,
//             )
//           }
//         }
//         t.MPend(body, signature) -> {
//           let (index, amount, timeout, image) = body
//           expect [pend, ..pend_cheques] = pend_cheques
//           expect (amount, timeout, image) == pend
//           expect verify_signature(key, tag, body, signature)
//           if index_bound < index {
//             fold_mix(
//               upper_bound,
//               key,
//               tag,
//               exclude,
//               tot + amount,
//               index,
//               mixs,
//               pends_amount - amount,
//               // TODO
//               pend_cheques,
//             )
//           } else {
//             fold_mix(
//               upper_bound,
//               key,
//               tag,
//               drop_hard(exclude, index_bound),
//               tot + amount,
//               index_bound,
//               mixs,
//               pends_amount,
//               // TODO
//               pend_cheques,
//             )
//           }
//         }
//       }
//   }
// }
