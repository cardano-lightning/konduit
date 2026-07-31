use crate::transaction::{
    BlockHeaderHash, BlockNo, InputIndex, KeyHash, Lovelace, OutputIndex, ScriptHash, SlotNo,
    TransactionId, TransactionIndex,
};
use cardano_sdk::VerificationKey;
use konduit_data::{Datum, Redeemer, Tag};
use rusqlite::Result;
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

pub struct SqlDatum(pub Datum);

impl FromSql for SqlDatum {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Blob(bytes) => Datum::try_from(bytes)
                .map_err(|e| FromSqlError::Other(Box::new(e)))
                .map(SqlDatum),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub struct SqlDatumRef<'a>(pub &'a Datum);

impl<'a> ToSql for SqlDatumRef<'a> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let bytes: Vec<u8> = self.0.into();
        Ok(ToSqlOutput::from(bytes))
    }
}

// pub struct SqlRedeemer(pub Redeemer);
// Let's transition to `Cow`
pub struct SqlRedeemer(pub Redeemer);

impl FromSql for SqlRedeemer {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Blob(bytes) => Redeemer::try_from(bytes)
                .map_err(|e| FromSqlError::Other(Box::new(e)))
                .map(SqlRedeemer),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub struct SqlRedeemerRef<'a>(pub &'a Redeemer);

impl ToSql for SqlRedeemerRef<'_> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let bytes: Vec<u8> = self.0.into();
        Ok(ToSqlOutput::from(bytes))
    }
}

pub struct SqlTag(pub Tag);

impl FromSql for SqlTag {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Blob(bytes) => Ok(SqlTag(Tag::from(bytes))),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

// We use `Cow` for things which are used in queries.
pub struct SqlTagRef<'a>(pub &'a Tag);

impl ToSql for SqlTagRef<'_> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let tag = self.0.as_ref();
        Ok(ToSqlOutput::from(tag))
    }
}

pub struct SqlVerificationKey(pub VerificationKey);

impl FromSql for SqlVerificationKey {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql32u8 = Sq32u8::column_result(value)?;
        Ok(SqlVerificationKey(VerificationKey::from(sql32u8.0)))
    }
}

pub struct SqlVerificationKeyRef<'a>(pub &'a VerificationKey);

impl ToSql for SqlVerificationKeyRef<'_> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        let bytes: &[u8] = self.0.as_ref();
        Ok(ToSqlOutput::from(bytes))
    }
}

// Extra helpers
pub struct Sq32u8([u8; 32]);

impl FromSql for Sq32u8 {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Blob(bytes) => {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(bytes);
                    Ok(Sq32u8(arr))
                } else {
                    Err(FromSqlError::InvalidType)
                }
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub struct Sql28u8Owned([u8; 28]);

impl FromSql for Sql28u8Owned {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Blob(bytes) => {
                if bytes.len() == 28 {
                    let mut arr = [0u8; 28];
                    arr.copy_from_slice(bytes);
                    Ok(Sql28u8Owned(arr))
                } else {
                    Err(FromSqlError::InvalidType)
                }
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

pub struct SqlU16(pub u16);

impl FromSql for SqlU16 {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => {
                if i >= 0 && i <= u16::MAX as i64 {
                    Ok(SqlU16(i as u16))
                } else {
                    Err(FromSqlError::InvalidType)
                }
            }
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for SqlU16 {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for BlockHeaderHash {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql32u8 = Sq32u8::column_result(value)?;
        Ok(BlockHeaderHash(sql32u8.0))
    }
}

impl ToSql for BlockHeaderHash {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_vec()))
    }
}

impl FromSql for BlockNo {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => Ok(BlockNo(i as u64)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for BlockNo {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for InputIndex {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql_u16 = SqlU16::column_result(value)?;
        Ok(InputIndex(sql_u16.0))
    }
}

impl ToSql for InputIndex {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for KeyHash {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql28u8 = Sql28u8Owned::column_result(value)?;
        Ok(KeyHash(sql28u8.0))
    }
}

impl ToSql for KeyHash {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_slice()))
    }
}

impl FromSql for Lovelace {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => Ok(Lovelace(i as u64)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}
impl ToSql for Lovelace {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for OutputIndex {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql_u16 = SqlU16::column_result(value)?;
        Ok(OutputIndex(sql_u16.0))
    }
}

impl ToSql for OutputIndex {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for SlotNo {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => Ok(SlotNo(i as u64)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl FromSql for ScriptHash {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql28u8 = Sql28u8Owned::column_result(value)?;
        Ok(ScriptHash(sql28u8.0))
    }
}

impl ToSql for ScriptHash {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_slice()))
    }
}

impl ToSql for SlotNo {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}

impl FromSql for TransactionId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let sql32u8 = Sq32u8::column_result(value)?;
        Ok(TransactionId(sql32u8.0))
    }
}

impl ToSql for TransactionId {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.as_slice()))
    }
}

impl FromSql for TransactionIndex {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        match value {
            ValueRef::Integer(i) => Ok(TransactionIndex(i as u64)),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

impl ToSql for TransactionIndex {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0 as i64))
    }
}
