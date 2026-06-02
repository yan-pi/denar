//! Surface-syntax frontend for btclisp: lexer, parser, and AST.
//!
//! The pipeline for v0,1 Sprint 1 is `source -> tokens -> Form AST`. Later
//! sprints add name resolution and lowering to `btclisp-core`'s `CoreExpr`.
//! This commit lands the AST, error type and lexer; the parser and lowering
//! follow.

pub mod ast;
pub mod error;
pub mod lex;

pub use ast::{Atom, Expr, Form};
pub use error::Error;
