//! Hash opcodes: `sha256`, `hash160`, `hash256`.
//!
//! Each concatenates its arguments, then hashes (spec §4):
//! - `sha256` — SHA-256 of the concatenation
//! - `hash256` — SHA-256(SHA-256(..)) (Bitcoin double-SHA)
//! - `hash160` — RIPEMD-160(SHA-256(..))

use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

use crate::error::EvalError;
use crate::value::Value;

/// Concatenate every argument's bytes, erroring on a cons argument.
fn concat(args: &[Value]) -> Result<Vec<u8>, EvalError> {
    let mut out = Vec::new();
    for arg in args {
        let bytes = arg
            .as_atom_bytes()
            .ok_or(EvalError::TypeError("hash expects atoms"))?;
        out.extend_from_slice(bytes);
    }
    Ok(out)
}

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

/// `(sha256 A B ...)` — SHA-256 of the concatenation.
pub(crate) fn sha256(args: &[Value]) -> Result<Value, EvalError> {
    Ok(Value::atom(sha256_bytes(&concat(args)?).to_vec()))
}

/// `(hash256 A B ...)` — double SHA-256 of the concatenation.
pub(crate) fn hash256(args: &[Value]) -> Result<Value, EvalError> {
    let once = sha256_bytes(&concat(args)?);
    Ok(Value::atom(sha256_bytes(&once).to_vec()))
}

/// `(hash160 A B ...)` — RIPEMD-160 of SHA-256 of the concatenation.
pub(crate) fn hash160(args: &[Value]) -> Result<Value, EvalError> {
    let sha = sha256_bytes(&concat(args)?);
    let ripemd: [u8; 20] = Ripemd160::digest(sha).into();
    Ok(Value::atom(ripemd.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hx(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256(&[]).unwrap(),
            Value::atom(hx(
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            ))
        );
        assert_eq!(
            sha256(&[Value::atom(b"abc".to_vec())]).unwrap(),
            Value::atom(hx(
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            ))
        );
    }

    #[test]
    fn hash256_double_sha_of_empty() {
        assert_eq!(
            hash256(&[]).unwrap(),
            Value::atom(hx(
                "5df6e0e2761359d30a8275058e299fcc0381534545f55cf43e41983f5d4c9456"
            ))
        );
    }

    #[test]
    fn hash160_of_empty() {
        assert_eq!(
            hash160(&[]).unwrap(),
            Value::atom(hx("b472a266d0bd89c13706a4132ccfb16f7c3b9fcb"))
        );
    }

    #[test]
    fn concatenates_before_hashing() {
        let split = sha256(&[Value::atom(b"ab".to_vec()), Value::atom(b"c".to_vec())]).unwrap();
        let whole = sha256(&[Value::atom(b"abc".to_vec())]).unwrap();
        assert_eq!(split, whole);
    }
}
