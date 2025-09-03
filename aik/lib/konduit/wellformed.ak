use aiken/cbor
use aiken/crypto.{
  Signature, VerificationKey, sha2_256, verify_ed25519_signature,
}
use aiken/primitive/bytearray
use konduit/prelude.{Amount, Index, Timestamp}
use konduit/types as t

pub fn verify_signature(
  key: VerificationKey,
  tag: t.Tag,
  body: Data,
  signature: Signature,
) {
  let message = bytearray.concat(tag, cbor.serialise(body))
  verify_ed25519_signature(key, message, signature)
}

pub fn verify_unlocked(
  ub: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  unlocked: t.Unlocked,
) {
  let (body, signature, preimage) = unlocked
  let (_, _, timeout, image) = body
  and {
    verify_signature(key, tag, body, signature)?,
    (ub < timeout)?,
    (sha2_256(preimage) == image)?,
  }
}

pub fn verify_receipt(
  ub: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  receipt: t.Receipt,
) {
  let (m_squash, unlockeds) = receipt
  when m_squash is {
    Some((squash_body, signature)) -> {
      expect verify_signature(key, tag, squash_body, signature)
      let (amount, index, exclude) = squash_body
      fold_unlocked(ub, key, tag, exclude, amount, index, unlockeds)
    }
    None -> fold_unlocked(ub, key, tag, [], 0, 0, unlockeds)
  }
}

/// Drop element or fail 
pub fn drop_hard(l: List<Index>, i: Index) {
  when l is {
    [] -> fail @"element must be in list"
    [fst, ..l] ->
      if fst == i {
        l
      } else {
        drop_hard(l, i)
      }
  }
}

pub fn fold_unlocked(
  ub: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  exclude: List<Index>,
  tot: Amount,
  index_bound: Index,
  unlockeds: List<t.Unlocked>,
) {
  when unlockeds is {
    [] -> tot
    [fst, ..unlockeds] -> {
      let (body, signature, preimage) = fst
      let (index, amount, timeout, image) = body
      expect and { // TODO
          verify_signature(key, tag, body, signature)?,
          (ub < timeout)?,
          (sha2_256(preimage) == image)?,
        }
      if index_bound < index {
        fold_unlocked(ub, key, tag, exclude, tot + amount, index, unlockeds)
      } else {
        fold_unlocked(
          ub,
          key,
          tag,
          drop_hard(exclude, index_bound), // TODO! 
          tot + amount,
          index_bound,
          unlockeds,
        )
      }
    }
  }
}

pub fn verify_m_receipt(
  ub: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  m_receipt: t.MReceipt,
  pends_amount: Amount,
  pend_cheques: List<t.PendCheque>,
) {
  let (m_squash, mixs) = m_receipt
  when m_squash is {
    Some((squash_body, signature)) -> {
      expect verify_signature(key, tag, squash_body, signature)
      let (tot, index, exclude) = squash_body
      fold_mix(
        ub,
        key,
        tag,
        exclude,
        tot,
        index,
        mixs,
        pends_amount,
        pend_cheques,
      )
    }
    None -> fold_mix(ub, key, tag, [], 0, 0, mixs, pends_amount, pend_cheques, )
  }
}

pub fn fold_mix(
  ub: Timestamp,
  key: VerificationKey,
  tag: t.Tag,
  exclude: List<Index>,
  tot: Amount,
  index_bound: Index,
  mixs: List<t.Mix>,
  pends_amount: Amount,
  pend_cheques: List<t.PendCheque>,
) {
  when mixs is {
    [] -> {
      expect pends_amount == 0
      expect pend_cheques == []
      tot
    }
    [fst, ..mixs] ->
      when fst is {
        t.MUnlocked(body, signature, preimage) -> {
          let (index, amount, timeout, image) = body
          expect verify_signature(key, tag, body, signature)
          expect (ub < timeout)
          expect sha2_256(preimage) == image
          if index_bound < index {
            fold_mix(
              ub,
              key,
              tag,
              exclude,
              tot + amount,
              index,
              mixs,
              pends_amount,
              pend_cheques,
            )
          } else {
            fold_mix(
              ub,
              key,
              tag,
              drop_hard(exclude, index_bound),
              tot + amount,
              index_bound,
              mixs,
              pends_amount,
              pend_cheques,
            )
          }
        }
        t.MPend(body, signature) -> {
          let (index, amount, timeout, image) = body
          expect [pend, ..pend_cheques] = pend_cheques
          expect (amount, timeout, image) == pend
          expect verify_signature(key, tag, body, signature)
          if index_bound < index {
            fold_mix(
              ub,
              key,
              tag,
              exclude,
              tot + amount,
              index,
              mixs,
              pends_amount - amount, // TODO
              pend_cheques,
            )
          } else {
            fold_mix(
              ub,
              key,
              tag,
              drop_hard(exclude, index_bound),
              tot + amount,
              index_bound,
              mixs,
              pends_amount, // TODO
              pend_cheques,
            )
          }
        }
      }
  }
}

