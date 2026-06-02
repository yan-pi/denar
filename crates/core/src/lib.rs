//! Core intermediate representation and opcode definitions for btclisp.
//!
//! This crate is dependency-free apart from [`bytes`] and is shared by the
//! `codec` and `vm` crates. It deliberately carries no source spans or
//! diagnostics — those live in the `compiler` crate's surface AST.
//!
//! # Example
//!
//! ```
//! use btclisp_core::{CoreExpr, Opcode};
//!
//! let expr = CoreExpr::cons(CoreExpr::atom(vec![Opcode::Add.as_byte()]), CoreExpr::Nil);
//! assert!(matches!(expr, CoreExpr::Cons(_, _)));
//! ```

pub mod ir;
pub mod opcode;

pub use ir::CoreExpr;
pub use opcode::Opcode;
