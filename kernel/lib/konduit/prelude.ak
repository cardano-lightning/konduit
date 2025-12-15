use aiken/interval.{IntervalBoundType}

/// Index is used to enumerate a collection
pub type Index =
  Int

/// Amount is used for value
pub type Amount =
  Int

/// Timestamp is used to for posix time
pub type Timestamp =
  Int

// Posix Timestamp
/// Type alias for Bytearray to indicate that this ought to be of a given length.
pub type Bytes32 =
  ByteArray

pub type Bytes64 =
  ByteArray

/// Signature is used where the value is the output of a sign function.
/// It is 64 bytes in length
pub type Signature =
  Bytes64

pub type Timebound =
  IntervalBoundType<Int>
