//! Arithmetic and comparison opcodes.
//!
//! Per spec §14.1, v0,1 uses pure `u64` little-endian arithmetic (CScriptNum
//! arrives in v0,2). `+`/`*` wrap modulo 2^64; `-` saturates at zero.

use crate::error::EvalError;
use crate::ops::{boolean, expect_arity};
use crate::value::Value;

/// `(+ A B ...)` — wrapping sum.
pub(crate) fn add(args: &[Value]) -> Result<Value, EvalError> {
    let mut acc = 0u64;
    for arg in args {
        acc = acc.wrapping_add(arg.as_u64()?);
    }
    Ok(Value::from_u64(acc))
}

/// `(- A B ...)` — `A - B - ...`, saturating at zero.
pub(crate) fn sub(args: &[Value]) -> Result<Value, EvalError> {
    let mut iter = args.iter();
    let mut acc = match iter.next() {
        Some(first) => first.as_u64()?,
        None => return Ok(Value::Nil),
    };
    for arg in iter {
        acc = acc.saturating_sub(arg.as_u64()?);
    }
    Ok(Value::from_u64(acc))
}

/// `(* A B ...)` — wrapping product.
pub(crate) fn mul(args: &[Value]) -> Result<Value, EvalError> {
    let mut acc = 1u64;
    for arg in args {
        acc = acc.wrapping_mul(arg.as_u64()?);
    }
    Ok(Value::from_u64(acc))
}

/// `(% A B)` — modulo.
pub(crate) fn rem(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("%", args, 2)?;
    let divisor = args[1].as_u64()?;
    if divisor == 0 {
        return Err(EvalError::DivByZero);
    }
    Ok(Value::from_u64(args[0].as_u64()? % divisor))
}

/// `(/% A B)` — `(quotient . remainder)`.
pub(crate) fn divmod(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("/%", args, 2)?;
    let divisor = args[1].as_u64()?;
    if divisor == 0 {
        return Err(EvalError::DivByZero);
    }
    let dividend = args[0].as_u64()?;
    Ok(Value::cons(
        Value::from_u64(dividend / divisor),
        Value::from_u64(dividend % divisor),
    ))
}

/// `(< A B ...)` — strictly ascending as unsigned little-endian integers.
pub(crate) fn lt(args: &[Value]) -> Result<Value, EvalError> {
    let nums = args
        .iter()
        .map(Value::as_u64)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(boolean(nums.windows(2).all(|w| w[0] < w[1])))
}

/// `(= A B ...)` — all arguments equal (`nil` equals the empty atom).
pub(crate) fn eq(args: &[Value]) -> Result<Value, EvalError> {
    let equal = match args.split_first() {
        Some((first, rest)) => rest.iter().all(|v| v == first),
        None => true,
    };
    Ok(boolean(equal))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: u64) -> Value {
        Value::from_u64(v)
    }

    #[test]
    fn addition_and_multiplication() {
        assert_eq!(add(&[n(2), n(3)]).unwrap(), n(5));
        assert_eq!(mul(&[n(4), n(5)]).unwrap(), n(20));
        assert_eq!(add(&[]).unwrap(), Value::Nil);
        assert_eq!(mul(&[]).unwrap(), n(1));
    }

    #[test]
    fn subtraction_saturates() {
        assert_eq!(sub(&[n(10), n(3), n(2)]).unwrap(), n(5));
        assert_eq!(sub(&[n(3), n(10)]).unwrap(), Value::Nil);
    }

    #[test]
    fn modulo_and_divmod() {
        assert_eq!(rem(&[n(7), n(3)]).unwrap(), n(1));
        assert_eq!(divmod(&[n(7), n(3)]).unwrap(), Value::cons(n(2), n(1)));
        assert_eq!(rem(&[n(7), n(0)]), Err(EvalError::DivByZero));
    }

    #[test]
    fn less_than() {
        assert_eq!(lt(&[n(1), n(2), n(3)]).unwrap(), Value::atom(vec![1]));
        assert_eq!(lt(&[n(1), n(1)]).unwrap(), Value::Nil);
    }

    #[test]
    fn equality() {
        assert_eq!(eq(&[n(5), n(5)]).unwrap(), Value::atom(vec![1]));
        assert_eq!(eq(&[n(5), n(6)]).unwrap(), Value::Nil);
    }
}
