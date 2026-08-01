use amaru_uplc::{
    arena::Arena,
    binder::{Binder, DeBruijn},
    data::PlutusData,
    machine::{EvalResult, PlutusVersion},
    program::Program,
    term::Term,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct AikenFn {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: ReturnType,
    #[serde(rename = "compiledCode")]
    pub compiled_code: String,
    pub hash: String,
    pub definitions: HashMap<String, Definition>,
}

#[derive(Debug, Deserialize)]
pub struct Parameter {
    pub title: String,
    pub schema: SchemaRef,
}

#[derive(Debug, Deserialize)]
pub struct ReturnType {
    pub title: String,
    pub schema: SchemaRef,
}

#[derive(Debug, Deserialize)]
pub struct SchemaRef {
    #[serde(rename = "$ref")]
    pub ref_: String,
}

#[derive(Debug, Deserialize)]
pub struct Definition {
    pub title: Option<String>,
    #[serde(rename = "dataType")]
    pub data_type: Option<String>,
    #[serde(rename = "anyOf")]
    pub any_of: Option<Vec<Constructor>>,
}

#[derive(Debug, Deserialize)]
pub struct Constructor {
    pub title: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    pub index: u32,
    pub fields: Vec<SchemaRef>,
}

impl AikenFn {
    pub fn from_file(path: &str) -> Self {
        let contents =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        contents
            .parse::<Self>()
            .unwrap_or_else(|e| panic!("failed to parse AikenFn: {e}"))
    }

    pub fn from_shortcut(name: &str) -> Self {
        let path = format!("{}/aiken-fn/{name}.json", env!("CARGO_MANIFEST_DIR"));
        Self::from_file(&path)
    }

    pub fn compiled_code_bytes(&self) -> Vec<u8> {
        hex::decode(&self.compiled_code)
            .unwrap_or_else(|e| panic!("invalid hex in compiledCode: {e}"))
    }

    /// FIXME :: Hard coded versioning!
    pub fn program<'a, V>(&self, arena: &'a Arena) -> &'a Program<'a, V>
    where
        V: Binder<'a>,
    {
        let bytes = self.compiled_code_bytes();
        let flat_bytes = decode_cbor_bytestring(&bytes).expect("failed to unwrap code");
        amaru_uplc::flat::decode::<V>(arena, &flat_bytes, PlutusVersion::V3, 1000).unwrap()
    }

    pub fn eval_with<T, F>(&self, value: &T, pred: F) -> bool
    where
        T: minicbor::Encode<()>,
        F: for<'a> FnOnce(&EvalResult<'a, DeBruijn>) -> bool,
    {
        let arena = Arena::new();
        let program = self.program::<DeBruijn>(&arena);
        let arg = Term::data(&arena, try_into_plutus_data(&arena, value).unwrap());
        let result = program.apply(&arena, arg).eval(&arena);
        println!("here {:?}", &result.term);
        pred(&result)
    }

    pub fn eval_true<T>(&self, value: &T) -> bool
    where
        T: minicbor::Encode<()>,
    {
        self.eval_with(value, is_true)
    }

    pub fn eval_false<T>(&self, value: &T) -> bool
    where
        T: minicbor::Encode<()>,
    {
        self.eval_with(value, is_false)
    }

    pub fn eval_err<T>(&self, value: &T) -> bool
    where
        T: minicbor::Encode<()>,
    {
        self.eval_with(value, is_err)
    }
}

impl std::str::FromStr for AikenFn {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

pub fn try_into_plutus_data<'a, T>(
    arena: &'a Arena,
    value: &T,
) -> Result<&'a PlutusData<'a>, String>
where
    T: minicbor::Encode<()>,
{
    let mut buf = Vec::new();
    minicbor::encode(value, &mut buf).unwrap();
    println!("minicbor bytes: {}", hex::encode(&buf));

    let result = PlutusData::from_cbor(arena, &buf).map_err(|e| e.to_string())?;
    println!("plutus data: {:?}", result);
    Ok(result)
}

pub fn is_true<'a>(result: &EvalResult<'a, DeBruijn>) -> bool {
    matches!(
        result.term,
        Ok(Term::Constant(amaru_uplc::constant::Constant::Boolean(
            true
        )))
    )
}

pub fn is_false<'a>(result: &EvalResult<'a, DeBruijn>) -> bool {
    matches!(
        result.term,
        Ok(Term::Constant(amaru_uplc::constant::Constant::Boolean(
            false
        )))
    )
}

pub fn is_err<'a>(result: &EvalResult<'a, DeBruijn>) -> bool {
    result.term.is_err()
}

fn decode_cbor_bytestring(bytes: &[u8]) -> Result<Vec<u8>, minicbor::decode::Error> {
    let mut decoder = minicbor::Decoder::new(bytes);
    decoder.bytes().map(|b| b.to_vec())
}
