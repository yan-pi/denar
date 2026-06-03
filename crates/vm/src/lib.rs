//! Eager interpreter for btclisp core expressions (spec §8).
//!
//! [`eval`] reduces a [`btclisp_core::CoreExpr`] program against a [`Value`]
//! environment, enforcing the budgets in [`Limits`]. Opcode handlers are
//! grouped by domain under the (private) `ops` module; hashing, signatures, and
//! transaction introspection arrive in Sprints 5–6.
//!
//! # Example
//!
//! ```
//! use btclisp_core::CoreExpr;
//! use btclisp_vm::{eval, EvalCtx, Value};
//!
//! // (q . 5) — quote returns its argument unevaluated.
//! let program = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![5]));
//! let mut ctx = EvalCtx::default();
//! assert_eq!(eval(&program, &Value::Nil, &mut ctx).unwrap(), Value::atom(vec![5]));
//! ```

pub mod error;
pub mod eval;
pub mod limits;
pub mod value;

pub(crate) mod ops;

pub use error::EvalError;
pub use eval::{eval, EvalCtx};
pub use limits::{Counters, Limits};
pub use ops::tx::TxContext;
pub use value::Value;
