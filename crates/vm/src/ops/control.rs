//! Boolean control opcodes: `not`, `all`, `any`.

use crate::error::EvalError;
use crate::ops::{boolean, expect_arity};
use crate::value::Value;

/// `(not X)` — logical negation by truthiness.
pub(crate) fn not(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("not", args, 1)?;
    Ok(boolean(!args[0].is_truthy()))
}

/// `(all A B ...)` — true if every argument is truthy.
pub(crate) fn all(args: &[Value]) -> Result<Value, EvalError> {
    Ok(boolean(args.iter().all(Value::is_truthy)))
}

/// `(any A B ...)` — true if any argument is truthy.
pub(crate) fn any(args: &[Value]) -> Result<Value, EvalError> {
    Ok(boolean(args.iter().any(Value::is_truthy)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_negates_truthiness() {
        assert_eq!(not(&[Value::Nil]).unwrap(), Value::atom(vec![1]));
        assert_eq!(not(&[Value::atom(vec![1])]).unwrap(), Value::Nil);
    }

    #[test]
    fn all_and_any() {
        let t = Value::atom(vec![1]);
        assert_eq!(all(&[t.clone(), t.clone()]).unwrap(), t);
        assert_eq!(all(&[t.clone(), Value::Nil]).unwrap(), Value::Nil);
        assert_eq!(any(&[Value::Nil, t.clone()]).unwrap(), t);
        assert_eq!(any(&[Value::Nil, Value::Nil]).unwrap(), Value::Nil);
    }
}
