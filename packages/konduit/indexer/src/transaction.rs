// Our call stacks are rather shallow
use cardano_sdk::address::kind::Shelley;
use cardano_sdk::{Address, PlutusData};
use derive_more::Display;
use itertools::{Either, EitherOrBoth, Itertools};
use konduit_data::{Datum, Redeemer};
use kupo_client::Match;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

#[derive(Debug, Display, Clone, PartialEq, Eq)]
#[display("{{ block_no: {block_no}, header_hash: {header_hash}, slot_no: {slot_no} }}")]
pub struct Block {
    pub block_no: BlockNo,
    pub header_hash: BlockHeaderHash,
    pub slot_no: SlotNo,
}

impl From<kupo_client::Checkpoint> for Block {
    fn from(checkpoint: kupo_client::Checkpoint) -> Self {
        Block {
            block_no: checkpoint.block_no.into(),
            header_hash: checkpoint.header_hash.into(),
            slot_no: checkpoint.slot_no.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockHeaderHash(pub [u8; 32]);

impl std::fmt::Display for BlockHeaderHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl From<kupo_client::Blake2b256> for BlockHeaderHash {
    fn from(hash: kupo_client::Blake2b256) -> Self {
        BlockHeaderHash(hash.0)
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct BlockNo(pub u64);

impl From<u64> for BlockNo {
    fn from(value: u64) -> Self {
        BlockNo(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub datum: Datum,
    pub input_index: InputIndex,
    pub lovelace: Lovelace,
    pub redeemer: Redeemer,
    pub tx_out_ref: TxOutRef,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputIndex(pub u16);

impl From<u16> for InputIndex {
    fn from(value: u16) -> Self {
        InputIndex(value)
    }
}

#[serde_as]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct KeyHash(#[serde_as(as = "serde_with::hex::Hex")] pub [u8; 28]);

#[serde_as]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Lovelace(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub datum: Datum,
    pub lovelace: Lovelace,
    pub output_index: OutputIndex,
    pub script_hash: ScriptHash,
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct OutputIndex(pub u16);

impl From<u16> for OutputIndex {
    fn from(value: u16) -> Self {
        OutputIndex(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptHash(pub [u8; 28]);

impl From<kupo_client::Blake2b224> for ScriptHash {
    fn from(hash: kupo_client::Blake2b224) -> Self {
        ScriptHash(hash.0)
    }
}

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SlotNo(pub u64);

impl From<u64> for SlotNo {
    fn from(value: u64) -> Self {
        SlotNo(value)
    }
}

impl SlotNo {
    pub fn succ(&self) -> Self {
        SlotNo(self.0 + 1)
    }
}

#[serde_as]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct TransactionId(#[serde_as(as = "serde_with::hex::Hex")] pub [u8; 32]);

impl From<&kupo_client::Blake2b256> for TransactionId {
    fn from(hash: &kupo_client::Blake2b256) -> Self {
        TransactionId(hash.0)
    }
}

impl From<kupo_client::Blake2b256> for TransactionId {
    fn from(hash: kupo_client::Blake2b256) -> Self {
        TransactionId(hash.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransactionIndex(pub u64);

impl From<u64> for TransactionIndex {
    fn from(value: u64) -> Self {
        TransactionIndex(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxOutRef(pub TransactionId, pub OutputIndex);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NonMutual {
    Cont(Box<(konduit_data::Cont, Output)>),
    Eol(konduit_data::Eol),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Steps {
    Batch(Vec<(Input, NonMutual)>),
    Mutual(Box<Input>),
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum InputDecodingError {
    #[error("Could not decode PlutusData for input: {0:?}: {1}")]
    DatumDecodingFailed(TxOutRef, String),
    #[error("Missing datum for input: {0:?}")]
    MissingDatum(TxOutRef),
    #[error("Missing spent_at for input: {0:?}")]
    MissingSpentAt(TxOutRef),
    // We flatten `minicbor::decode::Error` and `anyhow::Error`
    // into a string because it does not implement `Clone` etc.
    // which we would like to have.
    #[error("Could not decode redeemer for input: {0:?}: {1}")]
    RedeemerDecodingFailed(TxOutRef, String),
}

impl InputDecodingError {
    pub fn tx_out_ref(&self) -> TxOutRef {
        match self {
            InputDecodingError::DatumDecodingFailed(tx_out_ref, _)
            | InputDecodingError::MissingDatum(tx_out_ref)
            | InputDecodingError::MissingSpentAt(tx_out_ref)
            | InputDecodingError::RedeemerDecodingFailed(tx_out_ref, _) => *tx_out_ref,
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum OutputDecodingError {
    #[error("Missing datum for output: {0:?}")]
    MissingDatum(TxOutRef),
    #[error("Missing script hash for output: {tx_out_ref:?}: {kupo_match:?}")]
    MissingScriptHash {
        tx_out_ref: TxOutRef,
        kupo_match: Box<Match>,
    },
    #[error("Could not decode PlutusData for output: {0:?}: {1}")]
    PlutusDataDecodingFailed(TxOutRef, String),
}

impl OutputDecodingError {
    pub fn tx_out_ref(&self) -> TxOutRef {
        match self {
            OutputDecodingError::MissingDatum(tx_out_ref)
            | OutputDecodingError::MissingScriptHash { tx_out_ref, .. }
            | OutputDecodingError::PlutusDataDecodingFailed(tx_out_ref, _) => *tx_out_ref,
        }
    }
}

/// Decoding-pass errors.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum TransactionDecodingError {
    #[error("Cont step without a continuation output: {0:?}")]
    ContStepWithoutContinuation(Box<konduit_data::Cont>),
    #[error(transparent)]
    ContStepContinutionInvalid(OutputDecodingError),
    #[error("Given transaction has no steps and no opens")]
    EmptyKonduitTransaction,
    #[error("First redeemer in non-mutual transaction is not `Main`")]
    FirstRedeemerIsNotMain,
    #[error("Incompatible number of steps and inputs or outputs")]
    IncompatibleStepNumber,
    #[error(transparent)]
    InputDecodingFailed(#[from] InputDecodingError),
    #[error("Non-first redeemer in non-mutual transaction is not `Defer`")]
    NonFirstRedeemerIsNotDefer(TxOutRef),
    #[error(transparent)]
    OutputDecodingFailed(#[from] OutputDecodingError),
}

/// Invariant:
/// Either `steps` or `opens` is non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub block: Block,
    // TODO: Make this error output specific
    invalid_opens: Vec<OutputDecodingError>,
    opens: Vec<Output>,
    steps: Option<Steps>,
    pub transaction_id: TransactionId,
    pub transaction_index: TransactionIndex,
}

fn decode_block(inputs: &[Match], outputs: &[Match]) -> Result<Block, TransactionDecodingError> {
    match (
        outputs.first(),
        inputs.first().and_then(|m| m.spent_at.as_ref()),
    ) {
        (Some(first_output), _) => Ok(Block {
            block_no: BlockNo(first_output.created_at.block_no),
            header_hash: first_output.created_at.header_hash.into(),
            slot_no: SlotNo(first_output.created_at.slot_no),
        }),
        (_, Some(first_spent_at)) => Ok(Block {
            block_no: first_spent_at.block_no.into(),
            header_hash: first_spent_at.header_hash.into(),
            slot_no: SlotNo(first_spent_at.slot_no),
        }),
        (None, None) => Err(TransactionDecodingError::EmptyKonduitTransaction),
    }
}

fn decode_transaction_attrs(
    inputs: &[Match],
    outputs: &[Match],
) -> Result<(TransactionId, TransactionIndex), TransactionDecodingError> {
    match (
        outputs.first(),
        inputs.first().and_then(|m| m.spent_at.as_ref()),
    ) {
        (Some(first_output), _) => Ok((
            first_output.transaction_id.into(),
            first_output.transaction_index.into(),
        )),
        (_, Some(first_spent_at)) => Ok((
            first_spent_at.transaction_id.into(),
            first_spent_at.transaction_index.into(),
        )),
        (None, None) => Err(TransactionDecodingError::EmptyKonduitTransaction),
    }
}

fn decode_datum(bytes: &[u8]) -> std::result::Result<konduit_data::Datum, String> {
    let data: PlutusData = cardano_sdk::cbor::decode(bytes).map_err(|e| e.to_string())?;
    konduit_data::Datum::try_from(&data).map_err(|e| e.to_string())
}

fn decode_redeemer(bytes: Vec<u8>) -> std::result::Result<Redeemer, String> {
    let data: PlutusData = cardano_sdk::cbor::decode(&bytes).map_err(|e| e.to_string())?;
    Redeemer::try_from(&data).map_err(|e| e.to_string())
}

fn decode_input(m: Match) -> std::result::Result<Input, InputDecodingError> {
    let tx_out_ref = TxOutRef(m.transaction_id.into(), m.output_index.into());
    let datum_bytes = &m
        .datum
        .ok_or(InputDecodingError::MissingDatum(tx_out_ref))?;
    let datum = decode_datum(datum_bytes)
        .map_err(|e| InputDecodingError::DatumDecodingFailed(tx_out_ref, e))?;
    let lovelace = Lovelace(m.value.coins);
    let spent_at = m
        .spent_at
        .ok_or(InputDecodingError::MissingSpentAt(tx_out_ref))?;
    let redeemer = decode_redeemer(spent_at.redeemer)
        .map_err(|e| InputDecodingError::RedeemerDecodingFailed(tx_out_ref, e))?;
    let input_index = spent_at.input_index.into();
    let input = Input {
        datum,
        input_index,
        lovelace,
        redeemer,
        tx_out_ref,
    };
    Ok(input)
}

fn decode_output(output: Match) -> Result<Output, OutputDecodingError> {
    let mk_tx_out_ref = || TxOutRef(output.transaction_id.into(), output.output_index.into());
    let datum_bytes = output
        .datum
        .as_ref()
        .ok_or_else(|| OutputDecodingError::MissingDatum(mk_tx_out_ref()))?;
    let datum = decode_datum(datum_bytes)
        .map_err(|e| OutputDecodingError::PlutusDataDecodingFailed(mk_tx_out_ref(), e))?;
    let lovelace = Lovelace(output.value.coins);
    let script_hash = (|| {
        let address = <Address<Shelley>>::try_from(&output.address[..]).ok()?;
        let hash = address.payment().as_script()?;
        Some(ScriptHash(hash.into()))
    })()
    .ok_or_else(|| OutputDecodingError::MissingScriptHash {
        tx_out_ref: mk_tx_out_ref(),
        kupo_match: Box::new(output.clone()),
    })?;
    Ok(Output {
        datum,
        lovelace,
        output_index: output.output_index.into(),
        script_hash,
    })
}

fn decode_non_mutual(
    step: konduit_data::Step,
    outputs: &mut impl Iterator<Item = Result<Output, OutputDecodingError>>,
) -> Result<NonMutual, TransactionDecodingError> {
    match step {
        konduit_data::Step::Cont(cont) => match outputs.next() {
            Some(Ok(output)) => Ok(NonMutual::Cont(Box::new((cont, output)))),
            Some(Err(e)) => Err(TransactionDecodingError::ContStepContinutionInvalid(e)),
            None => Err(TransactionDecodingError::ContStepWithoutContinuation(
                Box::new(cont),
            )),
        },
        konduit_data::Step::Eol(eol) => Ok(NonMutual::Eol(eol)),
    }
}

fn decode_non_mutuals(
    first_input: Input,
    steps: Vec<konduit_data::Step>,
    inputs: &mut impl Iterator<Item = Input>,
    outputs: &mut impl Iterator<Item = Result<Output, OutputDecodingError>>,
) -> Result<Vec<(Input, NonMutual)>, TransactionDecodingError> {
    let mut steps = steps.into_iter();
    let first_step = steps
        .next()
        .ok_or(TransactionDecodingError::IncompatibleStepNumber)?;
    let first_non_mutual = decode_non_mutual(first_step, outputs)?;
    let first_entry = (first_input, first_non_mutual);

    let others = steps.zip_longest(inputs).map(|i| match i {
        EitherOrBoth::Both(step, input) => {
            if !matches!(input.redeemer, Redeemer::Defer) {
                return Err(TransactionDecodingError::NonFirstRedeemerIsNotDefer(
                    input.tx_out_ref,
                ));
            }
            let non_mutual = decode_non_mutual(step, outputs)?;
            Ok((input, non_mutual))
        }
        EitherOrBoth::Left(_) => Err(TransactionDecodingError::IncompatibleStepNumber),
        EitherOrBoth::Right(_) => Err(TransactionDecodingError::IncompatibleStepNumber),
    });
    std::iter::once(Ok(first_entry))
        .chain(others)
        .collect::<Result<Vec<(Input, NonMutual)>, TransactionDecodingError>>()
}

impl Transaction {
    // Accepts a pair of Kupo matches (inputs and outputs) and decodes them into a `Transaction`.
    // The matches can be provided in any order, but they must all belong to the same transaction.
    // Accepts a pair of Kupo matches (inputs and outputs) and decodes them into a `Transaction`.
    // The matches can be provided in any order, but they must all belong to the same transaction.
    pub fn from_kupo_matches(
        inputs: Vec<Match>,
        outputs: Vec<Match>,
    ) -> Result<Self, TransactionDecodingError> {
        let (transaction_id, transaction_index) = decode_transaction_attrs(&inputs, &outputs)?;
        let block = decode_block(&inputs, &outputs)?;

        // Decode, sort and prepare the inputs for further processing.
        let mut inputs = inputs
            .into_iter()
            .map(decode_input)
            .collect::<std::result::Result<Vec<Input>, InputDecodingError>>()?;
        inputs.sort_by_key(|input| input.input_index);
        let mut inputs = inputs.into_iter();

        type OutputDecodingResult = std::result::Result<Output, OutputDecodingError>;
        // Decode
        let mut outputs: Vec<(OutputDecodingResult, OutputIndex)> = outputs
            .into_iter()
            .map(|output| {
                let kupo_match = output.clone();
                let result = decode_output(output);
                (result, OutputIndex(kupo_match.output_index))
            })
            .collect::<Vec<(OutputDecodingResult, OutputIndex)>>();
        outputs.sort_by_key(|(_, output_index)| *output_index);
        let mut outputs = outputs.into_iter().map(|(result, _)| result);
        let steps: Option<Steps> = match inputs.next() {
            None => Ok(None),
            // We need a copy of the first redeemer as we preserve it
            // in both parts of the resulting `(input, non_mutual)` pair.
            Some(first_input) => match first_input.redeemer.clone() {
                Redeemer::Defer => Err(TransactionDecodingError::FirstRedeemerIsNotMain),
                Redeemer::Mutual => Ok(Some(Steps::Mutual(Box::new(first_input)))),
                Redeemer::Main(steps) => {
                    let non_mutuals =
                        decode_non_mutuals(first_input, steps, &mut inputs, &mut outputs)?;
                    Ok(Some(Steps::Batch(non_mutuals)))
                }
            },
        }?;
        let (invalid_opens, opens) = outputs.partition_map(|result| match result {
            Ok(output) => Either::Right(output),
            Err(e) => Either::Left(e),
        });

        Ok(Transaction {
            block,
            invalid_opens,
            opens,
            steps,
            transaction_index,
            transaction_id,
        })
    }

    pub fn steps(&self) -> Option<&Steps> {
        self.steps.as_ref()
    }

    pub fn opens(&self) -> &[Output] {
        &self.opens
    }
}
