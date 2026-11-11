use aiken/crypto.{VerificationKey}
use aiken/interval.{Finite}
use konduit/prelude.{Amount, Timebound}
use konduit/types as t

pub type SubVector {
  SubVector {
    name: ByteArray,
    seed: ByteArray,
    key: VerificationKey,
    tag: t.Tag,
    upper_bound: Timebound,
    useds_in: List<t.Used>,
    useds_out: List<t.Used>,
    squash: t.Squash,
    unlockeds: List<t.Unlocked>,
    expected: Amount,
    expected_return: VerificationKey,
    expect_failure: Bool,
    failure_reason: ByteArray,
  }
}

pub const vectors: List<SubVector> =
  [
    SubVector {
      name: "test_account_null",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [],
      useds_out: [],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [],
      expected: 0,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_account_scenario_0",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(1, 100), (3, 12)],
      useds_out: [(1, 100), (2, 100), (3, 12), (4, 12)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"0101010101010101010101010101010101010101010101010101010101010101",
          ),
          #"60a63aa59f69ad035f7acb6f41a8c05ed81b2736510f6d24955cbb1945d189cd13eabb678282e6b2a5b1e05b444fe734a028ca56728b09a64b6259c9e5ae0b0b",
        ),
        (
          (
            4,
            12,
            1800000000000,
            #"0202020202020202020202020202020202020202020202020202020202020202",
          ),
          #"d94ada21db36c423cccf769043aff86df74c4f34d6180d0e4ae42d05003104a9f6aa693fa285fbe6df33e7aad780110a9e101e7c13c47a3b29d1371ca413bc0c",
        ),
      ],
      expected: 224,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_account_scenario_1",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(1, 100), (3, 12)],
      useds_out: [(2, 100), (3, 12), (4, 12), (6, 111)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"0303030303030303030303030303030303030303030303030303030303030303",
          ),
          #"9bc15ef5e8a6fa17e8a709599002bb8dc390dca82b52239a8a196d248b704a571436257474cd6cf36b4f66eda21006c36bc675d78c0db4109e5601ef3af75e03",
        ),
        (
          (
            4,
            12,
            1800000000000,
            #"0404040404040404040404040404040404040404040404040404040404040404",
          ),
          #"76756b048c13b53c01cd92c290836107d719f202fc36e19b9fcf11cdf6b56ee3eecc7be020ae4fc5c7882f7ad9dec4a93232b937bac19ac7c81dc77ba2e5f30b",
        ),
        (
          (
            6,
            111,
            1800000000000,
            #"0505050505050505050505050505050505050505050505050505050505050505",
          ),
          #"e0d66c8dc32a0be7a9622ba9fe41f42c22cfe5f82f4fff0ea00634cdac8eae5e669a7522a0fe3e6ed42b5bb169a9c0a15c67ad481c46313668f9bb613c245d03",
        ),
      ],
      expected: 235,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_ooo_useds_outs",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(1, 100), (3, 12)],
      useds_out: [(3, 12), (2, 100), (4, 12), (6, 111)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"0606060606060606060606060606060606060606060606060606060606060606",
          ),
          #"3822736e0fbaf8aad8876e9228a03d98d8936fe161e493c9226aa74d33cca7c3f723ab9231365fffdf12ef7781a85be8c30647bb5ee4457a6549b8909b57e30d",
        ),
        (
          (
            4,
            12,
            1800000000000,
            #"0707070707070707070707070707070707070707070707070707070707070707",
          ),
          #"0850231462b9ac697c57212290fe63420c68a689fa1a0d824b8acb226fe6404611d0c3dff6e2b6d6b9f28c456b396d8ffc5cf84b4bc310558f7786271988480a",
        ),
        (
          (
            6,
            111,
            1800000000000,
            #"0808080808080808080808080808080808080808080808080808080808080808",
          ),
          #"058d92d4a63a87bc5f786b7c5ebc2b63a120f6e55c677295402d469cce9251c1c65bb6224ffe955422bb7751f1ddba0661530afc1ba94befc761c5326a95dc0b",
        ),
      ],
      expected: 235,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "useds_out is not strictly increasing",
    },
    SubVector {
      name: "test_ooo_useds_outs_1",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(1, 100), (3, 12)],
      useds_out: [(2, 100), (3, 12), (6, 111), (4, 12)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"0909090909090909090909090909090909090909090909090909090909090909",
          ),
          #"f2c473cc5098d396e2186231faf90dd9d87654ed82441fc6e1bcf6ed43abffb6bf3684ae4485113eac0f76e8e82e1c75e1cd9b9fa9b0965e69c3f994a2867f03",
        ),
        (
          (
            4,
            12,
            1800000000000,
            #"0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a",
          ),
          #"70cac2e8c4a0f89dc17e2a70c02307b2d0b05963552cee6a7063acfb5ecf5c29dd702b53e1472089dc175fd15b96667ad37ce2425c417aece7d0d2d0c8f0090a",
        ),
        (
          (
            6,
            111,
            1800000000000,
            #"0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
          ),
          #"7fadf4cf5d96deaffd3f824ffa124a21276afa64574b70f2c921decd26cd57f1fc3eb4738f7cfec5007fdbe7c6a9b4cc07f631c0aad6d3e1d809c56be0f5b10c",
        ),
      ],
      expected: 235,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "useds_out is not strictly increasing",
    },
    SubVector {
      name: "test_useds_out_too_long",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(1, 100), (3, 12)],
      useds_out: [(2, 100), (3, 12), (4, 12), (6, 111)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c",
          ),
          #"b15d2f1537b5c0e5cb4bacdd9653db4de7f1a2543c7823c375099da4f66b88cc035fcaaeac8ec0abf4e5d21ae1e1a333259c23e4ad64bd79bdb9d52bcfcd300f",
        ),
        (
          (
            4,
            12,
            1800000000000,
            #"0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d",
          ),
          #"8417c46729901f3e47ca651e3b4be7dff42dc7cd12c7f13a1ab0a9736529bd237a5c783a9ee3cc35d7aac0b4edc0c9f3948e0cbca2620e8766c5705b1e5b6b0b",
        ),
      ],
      expected: 235,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "missing unlocked for output index 6",
    },
    SubVector {
      name: "test_with_squash",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(1, 100), (3, 12)],
      useds_out: [(2, 100), (3, 12), (4, 12)],
      squash: (
        (1000, 1, []),
        #"533ea185be091d567994bd56f3f6db4bc7afcac41dac6c65652739a176c7a96a76e6bf45ef9a761830a6ead994ed64d72afda3e8d5e5cf7b58837efc60ea370b",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e",
          ),
          #"39e7a3d2d2ada4d919594a14c5bdfedb8a8f949426cca6eb1a4d30500692a04ea1a935634ce9a7c627c33e93b138d24b28d482479635888da23469f60006cb0b",
        ),
        (
          (
            4,
            12,
            1800000000000,
            #"0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f",
          ),
          #"a322c2e3722833b7200dd87d0f94b5d23b5b20ac7f04dacfdf92a187405a72d32b1be967b6cccffb4542289628109e38b13a4c52af705fd1a1c663ad77d54a03",
        ),
      ],
      expected: 1124,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_used_already_squashed",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(2, 100), (3, 12)],
      useds_out: [(2, 100), (3, 12), (4, 12)],
      squash: (
        (1000, 2, []),
        #"6a18f9bb771cb6a801fa80fd2e7e4b1c92ee720d88e802e4dc3d83e334e0467f04df1764009a7cf7faa1c208f26469e4b8ca3577bc974cbd28aafcde6092f60c",
      ),
      unlockeds: [
        (
          (
            4,
            12,
            1800000000000,
            #"1010101010101010101010101010101010101010101010101010101010101010",
          ),
          #"5edfffb18c698fd1a9cac26091f899373cb7b8d75989252e11c7f9e6eab6acdfa4e745d0d7a416d3f9f411f56418c9a80eb4f62aa193b46c2470ad1cee141c03",
        ),
      ],
      expected: 1124,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "persisted index is at or below squash index",
    },
    SubVector {
      name: "test_unlocked_timeout_too_early",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [],
      useds_out: [(1, 5)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            1,
            5,
            1799999999999,
            #"1818181818181818181818181818181818181818181818181818181818181818",
          ),
          #"055ca7691fe1aea425d0135a7db7c3ea1f9378c7c96a3416d116cb0069e1ddc57b23c3e1dd093eef8332f52a2078da840c0698687dcb6700c676572c9f65050b",
        ),
      ],
      expected: 5,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "unlocked timeout is below upper bound",
    },
    SubVector {
      name: "test_invalid_squash_signature",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [],
      useds_out: [],
      squash: (
        (0, 0, []),
        #"3114a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [],
      expected: 0,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "squash signature is invalid",
    },
    SubVector {
      name: "test_invalid_unlocked_signature",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [],
      useds_out: [(1, 5)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            1,
            5,
            1800000000000,
            #"1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b1b",
          ),
          #"089f5008c762a58462b5e5c051d852e5c5278fc96ff1f9dda204823a47e5c6fc06fa7dbfdc995ec8ad0bc374f5c9b0345a1ff2607361fbf4d08712783776b007",
        ),
      ],
      expected: 5,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "unlocked signature is invalid",
    },
    SubVector {
      name: "test_unlocked_mismatches_output",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [],
      useds_out: [(1, 5)],
      squash: (
        (0, 0, []),
        #"3014a5cf5d10eaa5df198dc8f0536162c67f90b10cd54fec76216cc4283cdc5cd9bdac8ad7d0a65da5cf41e9e32c481baa0be97c389c083f17133eae002a150c",
      ),
      unlockeds: [
        (
          (
            1,
            6,
            1800000000000,
            #"1919191919191919191919191919191919191919191919191919191919191919",
          ),
          #"5c540955051073f73f6898f7fd4758b61e001169a0d092ebcc4a8864c2ccf2a2765ecabe2509e2780f66a8f02a276d4bdd2c1706bf591c039ec82a9b3d5a1603",
        ),
      ],
      expected: 5,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: True,
      failure_reason: "unlocked amount does not match output",
    },
    SubVector {
      name: "test_excluded_index_is_skipped",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [],
      useds_out: [(2, 7)],
      squash: (
        (100, 3, [1, 2]),
        #"d4923edf0dbcc535f0d2f60698746ff3ce01a8d21c69fcfc5e4783bcbf281fdaee0a7524e03d702fd82ed798e9073c351751c63a2cdad6c7c396a42d8e81f20c",
      ),
      unlockeds: [
        (
          (
            2,
            7,
            1800000000000,
            #"1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a",
          ),
          #"3f6741fc0b4dd2e93e66fa877676ccc66f8c87e85b0818ab32412caa08b0d7eae3b7bd8c442a562a0d24010a9c779933e7eb13385637de8818d86f6fa51eea03",
        ),
      ],
      expected: 107,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_used_not_squashed",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(2, 100)],
      useds_out: [(2, 100), (4, 12)],
      squash: (
        (1000, 3, [2]),
        #"b9ca5c72953065b3566bfa3d11329cf170af8337b3d401d6bdd76e86b79cd177a85ddb795fd16a3c3c7a11bc1536c6a7340fab4229589790eb4fdd2d36adef08",
      ),
      unlockeds: [
        (
          (
            4,
            12,
            1800000000000,
            #"1111111111111111111111111111111111111111111111111111111111111111",
          ),
          #"232ed54dbd026d6ead15774e2e5fa01b0825084773e3519d06bdf5e5609410684473f7c8db758b948f94389bc8515bf021052f0a38b1ab75d8e5869b3957000d",
        ),
      ],
      expected: 1112,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_unlocked_already_used",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(2, 100)],
      useds_out: [(2, 100)],
      squash: (
        (1000, 3, [2]),
        #"b9ca5c72953065b3566bfa3d11329cf170af8337b3d401d6bdd76e86b79cd177a85ddb795fd16a3c3c7a11bc1536c6a7340fab4229589790eb4fdd2d36adef08",
      ),
      unlockeds: [
        (
          (
            2,
            100,
            1800000000000,
            #"1212121212121212121212121212121212121212121212121212121212121212",
          ),
          #"0d6e48145f1e25a892326ad81255759a5af7ace85ff07a5a497a48e65db0b2ce3cc617416cd9f7727b905088a82472405ca00f0bf192f32308a68ff3a183fe06",
        ),
      ],
      expected: 1100,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
    SubVector {
      name: "test_big_scenario",
      seed: #"0000000000000000000000000000000000000000000000000000000000000000",
      key: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      tag: #"6b6f6e647569742d636f72652d746573742d766563746f722d7631",
      upper_bound: Finite(1800000000000),
      useds_in: [(81, 1), (82, 1), (83, 1), (85, 1), (86, 1), (87, 1)],
      useds_out: [
        (80, 100), (82, 1), (84, 100), (86, 1), (88, 100), (92, 100), (96, 100),
      ],
      squash: (
        (10000000, 100, [80, 82, 84, 86, 88, 90, 92, 94, 96]),
        #"2204504e8478784c6e64c18f105731087bd5cce20cf31a4e25476493972a8e8f5f52d4588de50f08e4b1c9a9959daf4b5e8fed5bd545f6881c8c38b051a4b208",
      ),
      unlockeds: [
        (
          (
            80,
            100,
            1800000000000,
            #"1313131313131313131313131313131313131313131313131313131313131313",
          ),
          #"d1a8d0b352200314992b9a757b6d2c848f23313a5c6c40b4d15d34634abe6b94af3bbb0100229232f1c9a81c4f7f6196405a104a9893c67052956453fb65ca0e",
        ),
        (
          (
            84,
            100,
            1800000000000,
            #"1414141414141414141414141414141414141414141414141414141414141414",
          ),
          #"5e770f7dcc1b2ccb194d27c69853c68408db43acf570219ae3885704f21278b511cdac7025dd5cba2730b5e357d4ff8c00f719eeec080e2c0d6a9648cabada0b",
        ),
        (
          (
            88,
            100,
            1800000000000,
            #"1515151515151515151515151515151515151515151515151515151515151515",
          ),
          #"8cdfa10cb5008a38c1b3f3b36358822193fe3c571cd6e51854362ce926126e5b4e3278025fe74de0f128cd7caf1142631b4f4846aa88aba6ea6cee501377f304",
        ),
        (
          (
            92,
            100,
            1800000000000,
            #"1616161616161616161616161616161616161616161616161616161616161616",
          ),
          #"0de1c9028ca83c52abcd4263bdaa8d4970ccf29c587432eba2cc9f1e82d66ffadde6ed62535e43c0109c8b7359fa838dadcb61123c4a96cc53d21bf44f6a7b0b",
        ),
        (
          (
            96,
            100,
            1800000000000,
            #"1717171717171717171717171717171717171717171717171717171717171717",
          ),
          #"e1456e0b371704beec35d2758890a5626faee3b031e481ebbf5d5541056bf09b41175a1bfe43d510d1004fed44265c96e00fb74ed93997b4d5670e72bad1de0d",
        ),
      ],
      expected: 10000502,
      expected_return: #"3b6a27bcceb6a42d62a3a8d02a6f0d73653215771de243a63ac048a18b59da29",
      expect_failure: False,
      failure_reason: "",
    },
  ]
