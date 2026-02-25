//  This Source Code Form is subject to the terms of the Mozilla Public
//  License, v. 2.0. If a copy of the MPL was not distributed with this
//  file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Useful macros for testing and quickly constructing objects.

/// Construct variable-length [`Hash`](crate::Hash) from base16-encoded text strings.
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{hash};
/// assert_eq!(
///     <[u8; 28]>::from(hash!("00000000000000000000000000000000000000000000000000000000")),
///     [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0],
/// )
/// ```
///
/// ```rust
/// # use cardano_sdk::{hash};
/// assert_eq!(
///     <[u8; 32]>::from(hash!("702206530b2e1566e90b3aec753bd0abbf397842bd5421e0c3d23ed10167b3ce")),
///     [112, 34, 6, 83, 11, 46, 21, 102, 233, 11, 58, 236, 117, 59, 208, 171, 191, 57, 120, 66, 189, 84, 33, 224, 195, 210, 62, 209, 1, 103, 179, 206],
/// )
/// ```
#[macro_export]
macro_rules! hash {
    ($txt:literal $(,)?) => {
        <$crate::Hash<_>>::try_from($txt).unwrap()
    };
}

#[macro_export]
/// Construct [`Input`](crate::Input) from base16-encoded text strings & plain numbers.
///
/// The macro is variadic. It always requires a [`Hash<32>`](crate::Hash) for transaction id, and a
/// [`u64`] index. It may also take in an optional [`PlutusData`](crate::PlutusData) redeemer or
/// '_' to indicate none, in cases where the input is spending from a script.
macro_rules! input {
    ($id:literal, $index:expr $(,)?) => {
        $crate::Input::new(<$crate::Hash<32>>::try_from($id).unwrap(), $index)
    };

    ($id:literal, $index:expr, _ $(,)?) => {
        (
            $crate::Input::new(<$crate::Hash<32>>::try_from($id).unwrap(), $index),
            None,
        )
    };

    ($id:literal, $index:expr, $redeemer:expr $(,)?) => {
        (
            $crate::Input::new(<$crate::Hash<32>>::try_from($id).unwrap(), $index),
            Some($redeemer),
        )
    };
}

/// Construct a [`Mainnet`](crate::NetworkId::mainnet) address from a literal or from its
/// constituents.
///
/// Panics when given anything invalid.
///
/// Note that the first variation returns an [`Address<Any>`](crate::Address), whereas the two
/// other returns a [`Address<Shelley>`](crate::Address).
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{address, address::{Address, kind}};
/// // From a string literal:
/// let my_address: Address<kind::Any> =
///   address!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h");
/// ```
///
/// ```rust
/// # use cardano_sdk::{address, address::{Address, kind}, script_credential};
/// // From a script credential, using yet another macro:
/// assert_eq!(
///   address!(script_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777")).to_string(),
///   "addr1wx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twac57dez7",
/// );
/// ```
///
/// ```rust
/// # use cardano_sdk::{address, address::{Address, kind}, key_credential};
/// // From key credentials, with delegation:
/// assert_eq!(
///   address!(
///     key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"),
///     key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"),
///   ).to_string(),
///   "addr1qx7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twaaa8t5erdd2ln90uh98qavt6d4fktu89atld5apl7swkamst576s8",
/// );
/// ```
#[macro_export]
macro_rules! address {
    ($text:literal $(,)?) => {{
        let address = $crate::Address::<$crate::address::kind::Any>::try_from($text).unwrap();
        if address
            .as_shelley()
            .is_some_and(|shelley| shelley.network_id() != $crate::NetworkId::MAINNET)
        {
            panic!("network mismatch for address {}", $text);
        }
        address
    }};

    ($payment:expr $(,)?) => {
        $crate::Address::new($crate::NetworkId::MAINNET, $payment)
    };

    ($payment:expr, $delegation: expr $(,)?) => {
        $crate::Address::new($crate::NetworkId::MAINNET, $payment).with_delegation($delegation)
    };
}

/// Construct a [`Testnet`](crate::NetworkId::testnet) address from a literal or from its
/// constituents.
///
/// Panics when given anything invalid.
///
/// Note that the first variation returns an [`Address<Any>`](crate::Address), whereas the two
/// other returns a [`Address<Shelley>`](crate::Address).
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{address_test, address::{Address, kind}};
/// // From a string literal:
/// let my_address: Address<kind::Any> =
///   address_test!("addr_test1vr3gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds3vcc9j");
/// ```
///
/// ```rust
/// # use cardano_sdk::{address_test, address::{Address, kind}, script_credential};
/// // From a script credential, using yet another macro:
/// assert_eq!(
///   address_test!(script_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777")).to_string(),
///   "addr_test1wz7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twac0ke9dm",
/// );
/// ```
///
/// ```rust
/// # use cardano_sdk::{address_test, address::{Address, kind}, key_credential};
/// // From key credentials, with delegation:
/// assert_eq!(
///   address_test!(
///     key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"),
///     key_credential!("bd3ae991b5aafccafe5ca70758bd36a9b2f872f57f6d3a1ffa0eb777"),
///   ).to_string(),
///   "addr_test1qz7n46v3kk40ejh7tjnswk9ax65m97rj74lk6wsllg8twaaa8t5erdd2ln90uh98qavt6d4fktu89atld5apl7swkamsgzr6uc",
/// );
/// ```
#[macro_export]
macro_rules! address_test {
    ($text:literal $(,)?) => {{
        let address = $crate::Address::<$crate::address::kind::Any>::try_from($text).unwrap();
        if address
            .as_shelley()
            .is_some_and(|shelley| shelley.network_id() != $crate::NetworkId::TESTNET)
        {
            panic!("network mismatch for address {}", $text);
        }
        address
    }};

    ($payment:expr $(,)?) => {
        $crate::Address::new($crate::NetworkId::TESTNET, $payment)
    };

    ($payment:expr, $delegation: expr $(,)?) => {
        $crate::Address::new($crate::NetworkId::TESTNET, $payment).with_delegation($delegation)
    };
}

/// Construct a script [`Credential`](crate::Credential) from base16-encoded text strings.
///
/// See also:
///
/// - [`Credential::from_script`](crate::Credential::from_script)
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{script_credential};
/// assert!(
///     script_credential!("00000000000000000000000000000000000000000000000000000000")
///         .as_key()
///         .is_none()
/// )
/// ```
///
/// ```rust
/// # use cardano_sdk::{script_credential};
/// assert!(
///     script_credential!("00000000000000000000000000000000000000000000000000000000")
///         .as_script()
///         .is_some()
/// )
/// ```
#[macro_export]
macro_rules! script_credential {
    ($hash:literal $(,)?) => {
        $crate::Credential::from_script(<$crate::Hash<28>>::try_from($hash).unwrap())
    };
}

/// Construct a key [`Credential`](crate::Credential) from base16-encoded text strings.
///
/// See also:
///
/// - [`Credential::from_key`](crate::Credential::from_key)
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{key_credential};
/// assert!(
///     key_credential!("00000000000000000000000000000000000000000000000000000000")
///         .as_key()
///         .is_some()
/// )
/// ```
///
/// ```rust
/// # use cardano_sdk::{key_credential};
/// assert!(
///     key_credential!("00000000000000000000000000000000000000000000000000000000")
///         .as_script()
///         .is_none()
/// )
/// ```
#[macro_export]
macro_rules! key_credential {
    ($hash:literal $(,)?) => {
        $crate::Credential::from_key(<$crate::Hash<28>>::try_from($hash).unwrap())
    };
}

/// Construct an [`Output`](crate::Output) from an [`Address`](crate::Address) and an optional [`Value`](crate::Value).
///
/// See also:
///
/// - [`address!`](crate::address!)/[`address_test!`](crate::address_test!)
/// - [`value!`](crate::value!)
/// - [`Output::new`](crate::Output::new)
/// - [`Output::to`](crate::Output::to)
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{address, output};
/// # use indoc::indoc;
/// assert_eq!(
///   output!("addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h").to_string(),
///   indoc!{"
///     Output {
///         address: addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h,
///         value: Value {
///             lovelace: 857690,
///         },
///     }"
///   },
/// );
/// ```
///
/// ```rust
/// # use cardano_sdk::{address, output, value};
/// # use indoc::indoc;
/// assert_eq!(
///   output!(
///     "addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h",
///     value!(
///         123456789,
///         (
///             "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
///             "534e454b",
///             1,
///         ),
///     ),
///   ).to_string(),
///   indoc!{"
///     Output {
///         address: addr1v83gkkw3nqzakg5xynlurqcfqhgd65vkfvf5xv8tx25ufds2yvy2h,
///         value: Value {
///             lovelace: 123456789,
///             assets: {
///                 279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f: {
///                     SNEK: 1,
///                 },
///             },
///         },
///     }"
///   },
/// );
/// ```
#[macro_export]
macro_rules! output {
    ($addr:literal $(,)?) => {
        $crate::Output::to($crate::Address::try_from($addr).unwrap())
    };

    ($addr:literal, $value:expr $(,)?) => {
        $crate::Output::new($crate::Address::try_from($addr).unwrap(), $value)
    };
}

/// Construct a [`PlutusScript`](crate::PlutusScript) from a [`PlutusVersion`](crate::PlutusVersion) and a base16-encoded flat-serialised program.
///
/// See also:
///
/// - [`PlutusScript::new`](crate::PlutusScript::new)
///
/// # example
///
/// ```rust
/// # use cardano_sdk::{PlutusVersion, plutus_script};
/// let always_succeed = plutus_script!(PlutusVersion::V3, "5101010023259800a518a4d136564004ae69");
/// ```
#[macro_export]
macro_rules! plutus_script {
    ($lang:expr, $bytes:literal $(,)?) => {
        $crate::PlutusScript::new($lang, hex::decode($bytes).unwrap())
    };
}

/// Construct a [`PlutusData`](crate::PlutusData) from a serialised CBOR hex-encoded string.
///
/// See also:
///
/// - [`PlutusData::integer`](crate::PlutusData::integer)
/// - [`PlutusData::bytes`](crate::PlutusData::bytes)
/// - [`PlutusData::list`](crate::PlutusData::list)
/// - [`PlutusData::map`](crate::PlutusData::map)
/// - [`PlutusData::constr`](crate::PlutusData::constr)
#[macro_export]
macro_rules! plutus_data {
    ($bytes:literal $(,)?) => {
        $crate::cbor::decode::<$crate::PlutusData>(hex::decode($bytes).unwrap().as_slice()).unwrap()
    };
}

/// Construct a [`Value<u64>`](crate::Value) from a lovelace amount and a list of assets.
///
/// The macro is variadic. In its first form, it only accepts a lovelace quantity. In its second
/// second form, it accepts one or more asset as a triple.
///
/// The script hash and the asset name are expectd to be plain base16-encoded literals.
///
/// See also:
///
/// - [`assets!`](crate::assets) if you only want to create assets, without lovelace;
/// - [`Value::new`](crate::Value::new)
/// - [`Value::with_assets`](crate::Value::with_assets)
///
/// # examples
///
/// ```rust
/// # use cardano_sdk::{Value, hash, value};
/// assert_eq!(value!(123_456_789), Value::<u64>::new(123456789));
/// ```
///
/// ```rust
/// # use cardano_sdk::{Value, hash, value};
/// assert_eq!(
///     value!(
///         123_456_789,
///         (
///             "279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f",
///             "534e454b",
///             1_000_000,
///         ),
///     ),
///     Value::new(123456789)
///         .with_assets([
///             (
///                 hash!("279c909f348e533da5808898f87f9a14bb2c3dfbbacccd631d927a3f"),
///                 [( b"SNEK", 1_000_000)]
///             ),
///         ]),
/// );
/// ```
///
/// ```rust
/// # use cardano_sdk::{Value, hash, value};
/// assert_eq!(
///     value!(
///         0,
///         (
///             "b558ea5ecfa2a6e9701dab150248e94104402f789c090426eb60eb60",
///             "536e656b6b696530393033",
///             1,
///         ),
///         (
///             "b558ea5ecfa2a6e9701dab150248e94104402f789c090426eb60eb60",
///             "536e656b6b696533353536",
///             1,
///         ),
///         (
///             "a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235",
///             "484f534b59",
///             42_000_000,
///         )
///     ),
///     Value::default()
///         .with_assets([
///             (
///                 hash!("b558ea5ecfa2a6e9701dab150248e94104402f789c090426eb60eb60"),
///                 vec![( Vec::from(b"Snekkie0903"), 1), ( Vec::from(b"Snekkie3556"), 1)],
///             ),
///             (
///                 hash!("a0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235"),
///                 vec![( Vec::from(b"HOSKY"), 42_000_000)],
///             ),
///         ]),
/// );
/// ```
#[macro_export]
macro_rules! value {
    ($lovelace:expr $(,)?) => {
        $crate::Value::new($lovelace)
    };

    ($lovelace:expr, $( ($script_hash:literal, $asset_name:literal, $amount:expr $(,)?) ),+ $(,)? ) => {{
        $crate::Value::new($lovelace)
            .with_assets(vec![
                $(
                    (
                        <$crate::Hash<28>>::try_from($script_hash).unwrap(),
                        vec![ (hex::decode($asset_name).unwrap(), $amount) ],
                    )
                ),+
            ])
    }};
}

/// Construct a multi-asset object; akin to a [`Value<u64>`](crate::Value) but without lovelace.
///
/// The second variant of this macro takes an extra [`PlutusData`](crate::PlutusData) redeemer.
/// This is handy to create mint assets, which always come with a redeemer.
///
/// See also:
///
/// - [`value!`](crate::value)
/// - [`Transaction::with_mint`](crate::Transaction::with_mint)
#[macro_export]
macro_rules! assets {
    ($( ($script_hash:literal, $asset_name:literal, $amount:expr $(,)?) ),+ $(,)? ) => {{
        std::collections::BTreeMap::from([
            $(
                (
                    <$crate::Hash<28>>::try_from($script_hash).unwrap(),
                    std::collections::BTreeMap::from([ (hex::decode($asset_name).unwrap(), $amount) ]),
                )
            ),+
        ])
    }};

    ($( ($script_hash:literal, $asset_name:literal, $amount:expr, $redeemer:expr $(,)?) ),+ $(,)? ) => {{
        std::collections::BTreeMap::from([
            $(
                (
                    (<$crate::Hash<28>>::try_from($script_hash).unwrap(), $redeemer),
                    std::collections::BTreeMap::from([ (hex::decode($asset_name).unwrap(), $amount) ]),
                )
            ),+
        ])
    }};
}
