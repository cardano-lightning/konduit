// pub struct Instance {
//     cardano: Cardano,
//     contract_reference : OutputReference,
//     wallet_key: SigningKey,
// }

// pub async fn open(
//     instance
//     network: Cardano,
//     delegates: Vec<Hash<28>>,
//     choice: Vote,
//     anchor: Option<Anchor>,
//     proposal_id: GovActionId,
//     OutputReference(contract): OutputReference,
//     OutputReference(collateral): OutputReference,
// ) -> Tx {
//     let (validator, validator_hash, _) =
//         recover_validator(&network, &contract.transaction_id).await;
//
//     let params = network.protocol_parameters().await;
//
//     let resolved_inputs = network.resolve_many(&[&collateral, &contract]).await;
//     let fuel_output = expect_post_alonzo(&resolved_inputs[0].output);
//     let contract_output = expect_post_alonzo(&resolved_inputs[1].output);
//
//     let (rules, _) = recover_rules(&network, &validator_hash, &contract_output.value).await;
//
//     build_transaction(
//         &BuildParams::from(&params),
//         &resolved_inputs[..],
//         |fee, ex_units| {
//             let mut redeemers = vec![];
//
//             let inputs = vec![collateral.clone()];
//
//             let reference_inputs = vec![contract.clone()];
//
//             let outputs = vec![
//                 // Change
//                 PostAlonzoTransactionOutput {
//                     address: fuel_output.address.clone(),
//                     value: value_subtract_lovelace(fuel_output.value.clone(), fee)
//                         .expect("not enough fuel"),
//                     datum_option: None,
//                     script_ref: None,
//                 },
//             ];
//
//             let total_collateral = (fee as f64 * params.collateral_percent).ceil() as u64;
//
//             let collateral_return = PostAlonzoTransactionOutput {
//                 address: fuel_output.address.clone(),
//                 value: value_subtract_lovelace(fuel_output.value.clone(), total_collateral)
//                     .expect("not enough fuel"),
//                 datum_option: None,
//                 script_ref: None,
//             };
//
//             let votes = vec![(
//                 Voter::DRepScript(validator_hash),
//                 NonEmptyKeyValuePairs::Def(vec![(
//                     proposal_id.clone(),
//                     VotingProcedure {
//                         vote: choice.clone(),
//                         anchor: anchor.clone().map(Nullable::Some).unwrap_or(Nullable::Null),
//                     },
//                 )]),
//             )];
//             redeemers.push(Redeemer::vote(0, rules.clone(), ex_units[0]));
//
//             // ----- Put it all together
//             let redeemers = non_empty_pairs(redeemers).unwrap();
//             Tx {
//                 transaction_body: TransactionBody {
//                     inputs: Set::from(inputs),
//                     reference_inputs: non_empty_set(reference_inputs),
//                     network_id: Some(from_network(network.network_id())),
//                     outputs: into_outputs(outputs),
//                     voting_procedures: non_empty_pairs(votes),
//                     fee,
//                     collateral: non_empty_set(vec![fuel.clone()]),
//                     collateral_return: Some(PseudoTransactionOutput::PostAlonzo(collateral_return)),
//                     total_collateral: Some(total_collateral),
//                     required_signers: non_empty_set(delegates.clone()),
//                     script_data_hash: Some(
//                         script_integrity_hash(
//                             Some(&redeemers),
//                             None,
//                             &[(Language::PlutusV3, &params.cost_model_v3[..])],
//                         )
//                         .unwrap(),
//                     ),
//                     ..default_transaction_body()
//                 },
//                 transaction_witness_set: WitnessSet {
//                     redeemer: Some(redeemers.into()),
//                     plutus_v3_script: non_empty_set(vec![PlutusScript::<3>(validator.clone())]),
//                     ..default_witness_set()
//                 },
//                 success: true,
//                 auxiliary_data: Nullable::Null,
//             }
//         },
//     )
// }
