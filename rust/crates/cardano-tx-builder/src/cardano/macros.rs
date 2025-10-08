#[macro_export]
macro_rules! input {
    ($tx_hex:expr, $index:expr $(,)?) => {
        $crate::Input::new(<$crate::Hash<32>>::try_from($tx_hex).unwrap(), $index)
    };
}

#[macro_export]
macro_rules! output {
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

    // Lovelace + multi-assets as triples: (policy_id, asset_name, amount)
    ($lovelace:expr, $( ($policy_hex:expr, $asset_name:expr, $amount:expr $(,)?) ),+ $(,)? ) => {{
        $crate::Value::new($lovelace)
            .with_assets(vec![
                $(
                    (
                        <$crate::Hash<28>>::try_from($policy_hex).unwrap(),
                        vec![ (hex::decode($asset_name).unwrap(), $amount) ],
                    )
                ),+
            ])
    }};
}

#[macro_export]
macro_rules! mint {
    // multi-assets as a 4-tuple (policy_id, asset_name, amount, redeemer)
    ($( ($policy_hex:expr, $asset_name:expr, $amount:expr, $redeemer:expr $(,)?) ),+ $(,)? ) => {{
        std::collections::BTreeMap::from([
            $(
                (
                    (<$crate::Hash<28>>::try_from($policy_hex).unwrap(), $redeemer),
                    std::collections::BTreeMap::from([ (hex::decode($asset_name).unwrap(), $amount) ]),
                )
            ),+
        ])
    }};
}
