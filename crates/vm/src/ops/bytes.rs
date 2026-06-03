//! Byte-string opcodes: `cat`, `substr`, `strlen`, and the `rd`/`wr` codec pair.

use crate::error::EvalError;
use crate::ops::expect_arity;
use crate::value::Value;

/// `(cat A B ...)` — concatenate the argument byte strings.
pub(crate) fn cat(args: &[Value]) -> Result<Value, EvalError> {
    let mut out: Vec<u8> = Vec::new();
    for arg in args {
        let bytes = arg
            .as_atom_bytes()
            .ok_or(EvalError::TypeError("cat expects atoms"))?;
        out.extend_from_slice(bytes);
    }
    if out.is_empty() {
        Ok(Value::Nil)
    } else {
        Ok(Value::atom(out))
    }
}

/// `(substr A B E)` — the byte range `A[B..E]`.
pub(crate) fn substr(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("substr", args, 3)?;
    let bytes = args[0]
        .as_atom_bytes()
        .ok_or(EvalError::TypeError("substr expects an atom"))?;
    let start = usize::try_from(args[1].as_u64()?)
        .map_err(|_| EvalError::TypeError("substr index too large"))?;
    let end = usize::try_from(args[2].as_u64()?)
        .map_err(|_| EvalError::TypeError("substr index too large"))?;
    if start > end || end > bytes.len() {
        return Err(EvalError::TypeError("substr range out of bounds"));
    }
    let slice = &bytes[start..end];
    if slice.is_empty() {
        Ok(Value::Nil)
    } else {
        Ok(Value::atom(slice.to_vec()))
    }
}

/// `(strlen A B ...)` — the summed byte length of the arguments.
pub(crate) fn strlen(args: &[Value]) -> Result<Value, EvalError> {
    let mut total = 0u64;
    for arg in args {
        let bytes = arg
            .as_atom_bytes()
            .ok_or(EvalError::TypeError("strlen expects atoms"))?;
        total += bytes.len() as u64;
    }
    Ok(Value::from_u64(total))
}

/// `(wr A)` — serialize a value to its canonical byte encoding.
pub(crate) fn write(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("wr", args, 1)?;
    let bytes = btclisp_codec::encode(&args[0].to_core());
    Ok(Value::atom(bytes))
}

/// `(rd A)` — deserialize bytes back into a value.
pub(crate) fn read(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("rd", args, 1)?;
    let bytes = args[0]
        .as_atom_bytes()
        .ok_or(EvalError::TypeError("rd expects an atom"))?;
    let expr = btclisp_codec::decode(bytes)?;
    Ok(Value::from_core(&expr))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concatenation() {
        assert_eq!(
            cat(&[Value::atom(vec![0xaa]), Value::atom(vec![0xbb, 0xcc])]).unwrap(),
            Value::atom(vec![0xaa, 0xbb, 0xcc])
        );
        assert_eq!(cat(&[]).unwrap(), Value::Nil);
    }

    #[test]
    fn substring() {
        let s = Value::atom(vec![1, 2, 3, 4]);
        assert_eq!(
            substr(&[s.clone(), Value::from_u64(1), Value::from_u64(3)]).unwrap(),
            Value::atom(vec![2, 3])
        );
        assert!(substr(&[s, Value::from_u64(2), Value::from_u64(9)]).is_err());
    }

    #[test]
    fn length_sum() {
        assert_eq!(
            strlen(&[Value::atom(vec![1, 2]), Value::atom(vec![3])]).unwrap(),
            Value::from_u64(3)
        );
    }

    #[test]
    fn write_then_read_round_trips() {
        let value = Value::cons(Value::atom(vec![0x17]), Value::Nil);
        let written = write(std::slice::from_ref(&value)).unwrap();
        assert_eq!(read(&[written]).unwrap(), value);
    }
}
