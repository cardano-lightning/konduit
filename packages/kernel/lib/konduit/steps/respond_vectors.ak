use aiken/crypto.{VerificationKey}

pub const key: VerificationKey =
  #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29"

pub const tag: ByteArray =
  #"6b6f6e647569742d636f72652d746573742d766563746f722d7631"

pub const squash =
  (
    (0, 0, []),
    #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
  )

pub type RespondVector {
  RespondVector {
    name: ByteArray,
    upper_bound: Int,
    expected_return: VerificationKey,
  }
}

pub const vectors: List<RespondVector> =
  [
    RespondVector {
      name: "respond_empty_cheques",
      upper_bound: 1000,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
    },
  ]
