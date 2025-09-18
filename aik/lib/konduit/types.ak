use aiken/crypto.{ScriptHash, Signature, VerificationKey}
use konduit/prelude.{Amount, Bytes32, Hash32, Index, Timestamp}

/// CHEQUE 
pub type Secret =
  Bytes32

type Timeout =
  Timestamp

pub type ChequeBody =
  (Index, Amount, Timeout, Hash32)

pub type Cheque =
  (ChequeBody, Signature)

pub type Unlocked =
  (ChequeBody, Signature, Secret)

pub type Mix {
  MUnlocked(ChequeBody, Signature, Secret)
  MPend(ChequeBody, Signature)
}

/// SQUASH 
pub type Exclude =
  List<Index>

pub type SquashBody =
  (Amount, Index, Exclude)

pub type Squash =
  (SquashBody, Signature)

/// CONSTANT
pub type Tag =
  ByteArray

pub type Constants {
  tag: Tag,
  add_vkey: VerificationKey,
  sub_vkey: VerificationKey,
  close_period: Int,
}

/// RECEIPT 
pub type Receipt =
  (Squash, List<Unlocked>)

pub type MReceipt =
  (Squash, List<Mix>)

/// STAGE
type ElapseAt =
  Timestamp

pub type PendCheque =
  (Amount, Timeout, Hash32)

/// Total amount in pending cheques, and the PendCheques
pub type Pends =
  (Amount, List<PendCheque>)

pub type Stage {
  Opened(Amount)
  Closed(Amount, ElapseAt)
  Responded(Amount, List<PendCheque>)
}

/// DATUM 
/// ScriptHash is own_hash
pub type Datum =
  (ScriptHash, Constants, Stage)

pub type Redeemer {
  Batch
  Main(Steps)
  Mutual
}

pub type Step {
  StepCont(Cont)
  StepEol(Eol)
}

pub type Steps =
  List<Step>

pub type Unpend =
  ByteArray

pub type Cont {
  Add
  Sub(Squash, List<Unlocked>)
  Close
  Respond(Squash, List<Mix>)
  Unlock(List<Unpend>)
  Expire(List<Unpend>)
}

pub type Eol {
  End
  Elapse
}
