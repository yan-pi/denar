//! Surface-syntax frontend for btclisp: lexer, parser, and AST.
//!
//! The pipeline for v0,1 Sprint 1 is `source -> tokens -> Form AST`. Later
//! sprints add name resolution and lowering to `btclisp-core`'s `CoreExpr`.
//!
//! # Example
//!
//! ```
//! let forms = btclisp_compiler::parse_program("(+ 1 2)").unwrap();
//! assert_eq!(forms.len(), 1);
//! assert_eq!(forms[0].to_string(), "(+ 1 2)");
//! ```

pub mod ast;
pub mod error;
pub mod lex;
pub mod parse;

pub use ast::{Atom, Expr, Form};
pub use error::Error;

/// Lex and parse a complete source program into top-level [`Form`]s.
///
/// # Errors
///
/// Returns the first lexing or parsing [`Error`] encountered.
pub fn parse_program(src: &str) -> Result<Vec<Form>, Error> {
    let tokens = lex::lex(src)?;
    parse::Parser::new(tokens).parse_program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_source_to_no_forms() {
        assert_eq!(parse_program("   ; just a comment\n").unwrap(), Vec::new());
    }

    #[test]
    fn surfaces_lex_errors() {
        assert!(parse_program("0xZZ").is_err());
    }
}
