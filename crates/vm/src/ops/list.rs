//! List opcodes: `c` (cons), `h` (head), `t` (tail), `l` (list?), `b` (bintree).

use crate::error::EvalError;
use crate::ops::{boolean, expect_arity};
use crate::value::Value;

/// `(c H T)` — construct a cons cell.
pub(crate) fn cons(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("c", args, 2)?;
    Ok(Value::cons(args[0].clone(), args[1].clone()))
}

/// `(h L)` — the head of a cons cell.
pub(crate) fn head(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("h", args, 1)?;
    match &args[0] {
        Value::Cons(pair) => Ok(pair.0.clone()),
        _ => Err(EvalError::HeadOfAtom),
    }
}

/// `(t L)` — the tail of a cons cell.
pub(crate) fn tail(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("t", args, 1)?;
    match &args[0] {
        Value::Cons(pair) => Ok(pair.1.clone()),
        _ => Err(EvalError::TailOfAtom),
    }
}

/// `(l X)` — is `X` a cons cell?
pub(crate) fn is_list(args: &[Value]) -> Result<Value, EvalError> {
    expect_arity("l", args, 1)?;
    Ok(boolean(matches!(args[0], Value::Cons(_))))
}

/// `(b A B ...)` — a balanced cons tree of the arguments.
pub(crate) fn bintree(args: &[Value]) -> Result<Value, EvalError> {
    Ok(build_balanced(args))
}

fn build_balanced(args: &[Value]) -> Value {
    match args.len() {
        0 => Value::Nil,
        1 => args[0].clone(),
        n => {
            let mid = n / 2;
            Value::cons(build_balanced(&args[..mid]), build_balanced(&args[mid..]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cons_head_tail() {
        let pair = cons(&[Value::atom(vec![1]), Value::atom(vec![2])]).unwrap();
        assert_eq!(
            head(std::slice::from_ref(&pair)).unwrap(),
            Value::atom(vec![1])
        );
        assert_eq!(
            tail(std::slice::from_ref(&pair)).unwrap(),
            Value::atom(vec![2])
        );
    }

    #[test]
    fn head_of_atom_errors() {
        assert_eq!(head(&[Value::atom(vec![1])]), Err(EvalError::HeadOfAtom));
    }

    #[test]
    fn list_predicate() {
        let pair = Value::cons(Value::Nil, Value::Nil);
        assert_eq!(is_list(&[pair]).unwrap(), Value::atom(vec![1]));
        assert_eq!(is_list(&[Value::atom(vec![1])]).unwrap(), Value::Nil);
    }

    #[test]
    fn bintree_is_balanced() {
        let args = vec![
            Value::atom(vec![1]),
            Value::atom(vec![2]),
            Value::atom(vec![3]),
        ];
        // (b 1 2 3) -> (1 . (2 . 3))
        let expected = Value::cons(
            Value::atom(vec![1]),
            Value::cons(Value::atom(vec![2]), Value::atom(vec![3])),
        );
        assert_eq!(bintree(&args).unwrap(), expected);
    }
}
