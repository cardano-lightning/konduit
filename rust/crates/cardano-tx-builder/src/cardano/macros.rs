#[macro_export]
macro_rules! input {
    ($id:literal, $index:expr $(,)?) => {
        (
            $crate::Input::new(<$crate::Hash<32>>::try_from($id).unwrap(), $index),
            None::<$crate::PlutusData>,
        )
    };

    ($id:literal, $index:literal, $redeemer:expr $(,)?) => {
        (
            $crate::Input::new(<$crate::Hash<32>>::try_from($tx_hex).unwrap(), $index),
            Some($redeemer),
        )
    };
}

#[macro_export]
macro_rules! address {
    ($text:literal $(,)?) => {
        $crate::Address::try_from($text).unwrap()
    };

    ($payment:expr $(,)?) => {
        $crate::Address::new($crate::NetworkId::mainnet(), $payment)
    };

    ($network:expr, $payment:expr, $delegation: expr $(,)?) => {
        $crate::Address<_, $crate::address::Any>::from(
            $crate::Address::new($crate::NetworkId::mainnet(), $payment).with_delegation($delegation)
        )
    };
}

#[macro_export]
macro_rules! address_test {
    ($text:literal $(,)?) => {
        $crate::Address::try_from($text).unwrap()
    };

    ($payment:expr $(,)?) => {
        $crate::Address::new($crate::NetworkId::testnet(), $payment)
    };

    ($network:expr, $payment:expr, $delegation: expr $(,)?) => {
        $crate::Address<_, $crate::address::Any>::from(
            $crate::Address::new($crate::NetworkId::testnet(), $payment).with_delegation($delegation)
        )
    };
}

#[macro_export]
macro_rules! script_credential {
    ($hash:literal $(,)?) => {
        $crate::Credential::from_script(<$crate::Hash<28>>::try_from($hash).unwrap())
    };
}

#[macro_export]
macro_rules! key_credential {
    ($hash:literal $(,)?) => {
        $crate::Credential::from_key(hex::decode($hash).unwrap())
    };
}

#[macro_export]
macro_rules! output {
    ($addr:expr $(,)?) => {
        $crate::Output::to($crate::Address::try_from($addr).unwrap())
    };

    ($addr:expr, $value:expr $(,)?) => {
        $crate::Output::new($crate::Address::try_from($addr).unwrap(), $value)
    };
}

#[macro_export]
macro_rules! plutus_script {
    ($lang:expr, $bytes:expr $(,)?) => {
        $crate::PlutusScript::new($lang, hex::decode($bytes).unwrap())
    };
}

#[macro_export]
macro_rules! value {
    // Just lovelace
    ($lovelace:expr $(,)?) => {
        $crate::Value::new($lovelace)
    };

    // Lovelace + multi-assets as triples: (script_hash, asset_name, amount)
    ($lovelace:expr, $( ($script_hash:expr, $asset_name:expr, $amount:expr $(,)?) ),+ $(,)? ) => {{
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

#[macro_export]
macro_rules! assets {
    // multi-assets as a 3-tuple (script_hash, asset_name, amount)
    ($( ($script_hash:expr, $asset_name:expr, $amount:expr $(,)?) ),+ $(,)? ) => {{
        std::collections::BTreeMap::from([
            $(
                (
                    <$crate::Hash<28>>::try_from($script_hash).unwrap(),
                    std::collections::BTreeMap::from([ (hex::decode($asset_name).unwrap(), $amount) ]),
                )
            ),+
        ])
    }};

    // multi-assets as a 4-tuple (script_hash, asset_name, amount, redeemer)
    ($( ($script_hash:expr, $asset_name:expr, $amount:expr, $redeemer:expr $(,)?) ),+ $(,)? ) => {{
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
