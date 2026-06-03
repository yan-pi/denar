//! The runtime [`Value`] type and its conversions to/from [`CoreExpr`].

use std::rc::Rc;

use btclisp_core::CoreExpr;
use bytes::Bytes;

use crate::error::EvalError;

/// A runtime value: the empty list, a byte-string atom, or a cons cell.
///
/// [`Value`] mirrors [`CoreExpr`] but uses reference-counted cons cells for
/// cheap sharing during evaluation. Equality treats `nil` and the empty atom as
/// equal (both denote the empty byte string).
#[derive(Clone, Debug)]
pub enum Value {
    /// The empty list / empty atom.
    Nil,
    /// A byte-string atom.
    Atom(Bytes),
    /// A cons cell `(head . tail)`.
    Cons(Rc<(Value, Value)>),
}

impl Value {
    /// Construct an atom from anything convertible into [`Bytes`].
    #[must_use]
    pub fn atom(bytes: impl Into<Bytes>) -> Self {
        Value::Atom(bytes.into())
    }

    /// Construct a cons cell.
    #[must_use]
    pub fn cons(head: Value, tail: Value) -> Self {
        Value::Cons(Rc::new((head, tail)))
    }

    /// Convert a core expression (program data) into a runtime value.
    #[must_use]
    pub fn from_core(expr: &CoreExpr) -> Self {
        match expr {
            CoreExpr::Nil => Value::Nil,
            CoreExpr::Atom(bytes) => Value::Atom(bytes.clone()),
            CoreExpr::Cons(head, tail) => {
                Value::cons(Value::from_core(head), Value::from_core(tail))
            }
        }
    }

    /// Convert a runtime value back into a core expression.
    #[must_use]
    pub fn to_core(&self) -> CoreExpr {
        match self {
            Value::Nil => CoreExpr::Nil,
            Value::Atom(bytes) => CoreExpr::Atom(bytes.clone()),
            Value::Cons(pair) => CoreExpr::cons(pair.0.to_core(), pair.1.to_core()),
        }
    }

    /// Render this value as an s-expression (via [`CoreExpr::to_sexpr`]).
    #[must_use]
    pub fn to_sexpr(&self) -> String {
        self.to_core().to_sexpr()
    }

    /// Returns `false` for `nil`/empty atom, `true` otherwise.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Atom(bytes) => !bytes.is_empty(),
            Value::Cons(_) => true,
        }
    }

    /// The atom bytes of this value, treating `nil` as the empty slice.
    ///
    /// Returns `None` for cons cells.
    #[must_use]
    pub fn as_atom_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Nil => Some(&[]),
            Value::Atom(bytes) => Some(bytes),
            Value::Cons(_) => None,
        }
    }

    /// Interpret this value as an unsigned little-endian integer (max 8 bytes).
    ///
    /// # Errors
    ///
    /// Returns [`EvalError::TypeError`] for cons cells or atoms wider than 8
    /// bytes.
    pub fn as_u64(&self) -> Result<u64, EvalError> {
        let bytes = self
            .as_atom_bytes()
            .ok_or(EvalError::TypeError("expected an atom, found a list"))?;
        le_to_u64(bytes)
    }

    /// Build a value from an unsigned integer (minimal little-endian; 0 = nil).
    #[must_use]
    pub fn from_u64(n: u64) -> Self {
        let bytes = le_minimal(n);
        if bytes.is_empty() {
            Value::Nil
        } else {
            Value::atom(bytes)
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Cons(a), Value::Cons(b)) => a.0 == b.0 && a.1 == b.1,
            (Value::Cons(_), _) | (_, Value::Cons(_)) => false,
            (left, right) => left.as_atom_bytes() == right.as_atom_bytes(),
        }
    }
}

impl Eq for Value {}

/// Decode up to 8 little-endian bytes into a `u64`.
///
/// # Errors
///
/// Returns [`EvalError::TypeError`] if more than 8 bytes are supplied.
pub fn le_to_u64(bytes: &[u8]) -> Result<u64, EvalError> {
    if bytes.len() > 8 {
        return Err(EvalError::TypeError("integer exceeds 64 bits"));
    }
    let mut buf = [0u8; 8];
    buf[..bytes.len()].copy_from_slice(bytes);
    Ok(u64::from_le_bytes(buf))
}

/// Minimal little-endian byte encoding of `n` (trailing zero bytes stripped).
#[must_use]
pub fn le_minimal(n: u64) -> Vec<u8> {
    let mut bytes = n.to_le_bytes().to_vec();
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_core() {
        let value = Value::cons(Value::atom(vec![1, 2]), Value::Nil);
        assert_eq!(Value::from_core(&value.to_core()), value);
    }

    #[test]
    fn nil_equals_empty_atom() {
        assert_eq!(Value::Nil, Value::atom(Vec::<u8>::new()));
    }

    #[test]
    fn truthiness() {
        assert!(!Value::Nil.is_truthy());
        assert!(!Value::atom(Vec::<u8>::new()).is_truthy());
        assert!(Value::atom(vec![0]).is_truthy());
        assert!(Value::cons(Value::Nil, Value::Nil).is_truthy());
    }

    #[test]
    fn integer_round_trip() {
        for n in [0u64, 1, 255, 256, 65_535, u64::MAX] {
            assert_eq!(Value::from_u64(n).as_u64().unwrap(), n);
        }
        assert_eq!(Value::from_u64(0), Value::Nil);
    }
}
