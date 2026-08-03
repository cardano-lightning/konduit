use aiken/crypto.{ScriptHash, Signature, VerificationKey}
use konduit/prelude.{Amount, Bytes32, Index, Timestamp}

/// Bytes used as either the secret (preimage)
/// or the lock (SHA2-256 hash of the secret).
/// Both have identical length.
pub type Latch =
  Bytes32

pub type Lock =
  Latch

pub type Secret =
  Latch

/// Indicates the timeout, aka expiration of the cheque.
/// It cannot be used as evidence after the timeout 
/// has passed
type Timeout =
  Timestamp

/// Cheque body
/// Index must strictly monotonically increase in time.
/// Locked : Latch = Lock
/// Unlocked : Latch = Secret
pub type ChequeBody =
  (Index, Amount, Timeout, Latch)

/// Locked
/// The signature corresponds that of the
/// add_vkey on the messaged from of the "tagged" cheque body
/// Note this is never used.
/// pub type Locked = (ChequeBody, Signature)
/// Unlocked
/// An unlocked cheque is valid only if the signature verifies against the
/// body formed by replacing Latch (the Secret) with its SHA2-256 hash (the Lock).
pub type Unlocked =
  (ChequeBody, Signature)

/// A mix of unlocked or (locked) cheques
pub type Cheque {
  UnlockedCheque(ChequeBody, Signature)
  LockedCheque(ChequeBody, Signature)
}

/// Exclude list
pub type Exclude =
  List<Index>

/// The squash is evidence that "squashes" all cheques resolved.
/// Amount indicates total amount squashed (since chanel instantiation)
/// Index includes the greatest index of cheque squashed.
/// The exclude list should be monotonically strictly increasing, 
/// and only contain ints x: 0 < x < index. 
/// These correspond to cheques not squashed.
pub type SquashBody =
  (Amount, Index, Exclude)

pub type Squash =
  (SquashBody, Signature)

/// Tag of a channel instance. 
/// This is chosen by the opener. 
/// It is used to tag evidence. 
/// It is up to the opener/add_vkey owner
/// not to unsafely reuse the pair (tag,vkey).
pub type Tag =
  ByteArray

/// Constants
/// These are held in the datum but persist for the full instance lifecycle.
/// ```aiken
///    let (tag, add_vkey, sub_vkey, close_period) = constants
/// ```
pub type Constants =
  (Tag, VerificationKey, VerificationKey, Int)

/// STAGE
type ElapseAt =
  Timestamp

pub type Used =
  (Index, Amount)

pub type Pending =
  (Amount, Timeout, Lock)

pub type Stage {
  Opened(Amount, List<Used>)
  Closed(Amount, List<Used>, ElapseAt)
  Responded(Amount, List<Pending>)
}

/// DATUM 
/// ScriptHash is own_hash
pub type Datum =
  (ScriptHash, Constants, Stage)

pub type Redeemer {
  Defer
  Main(List<Step>)
  Mutual
}

pub type Step {
  StepCont(Cont)
  StepEol(Eol)
}

pub type Unpend =
  ByteArray

pub type Cont {
  Add
  Sub(Squash, List<Unlocked>)
  Close
  Respond(Squash, List<Cheque>)
  Unlock(List<Unpend>)
  Expire(List<Unpend>)
}

pub type Eol {
  End
  Elapse
}
