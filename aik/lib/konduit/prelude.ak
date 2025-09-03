use aiken/crypto.{VerificationKeyHash}
use aiken/interval.{IntervalBound}

pub type Index =
  Int

pub type Amount =
  Int

/// Posix Timestamp
pub type Timestamp =
  Int

/// 32 bytes
pub type Bytes32 =
  ByteArray

pub type Hash32 =
  Bytes32

/// 64 Bytes
pub type Signature =
  ByteArray

pub type Signers =
  List<VerificationKeyHash>

pub type Bound =
  IntervalBound<Int>
