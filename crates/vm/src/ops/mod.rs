//! Opcode handler groups (spec §2).
//!
//! Control flow (`q`/`a`/`i`/`x`) is handled directly in [`crate::eval`]; the
//! remaining strict opcodes live here, grouped by domain.

pub(crate) mod arith;
pub(crate) mod bytes;
pub(crate) mod control;
pub(crate) mod crypto;
pub(crate) mod hash;
pub(crate) mod list;
pub(crate) mod tx;

use crate::value::Value;

/// Build the canonical boolean value: `0x01` for true, `nil` for false.
#[must_use]
pub(crate) fn boolean(b: bool) -> Value {
    if b {
        Value::atom(vec![1])
    } else {
        Value::Nil
    }
}

/// Require exactly `n` arguments, returning an arity error otherwise.
pub(crate) fn expect_arity(
    op: &'static str,
    args: &[Value],
    n: usize,
) -> Result<(), crate::error::EvalError> {
    if args.len() == n {
        Ok(())
    } else {
        Err(crate::error::EvalError::Arity {
            op,
            expected: n,
            found: args.len(),
        })
    }
}
