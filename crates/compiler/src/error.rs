//! The single public error type for the compiler frontend.

use crate::ast::Span;

/// An error produced while lexing or parsing btclisp source.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    /// A run of characters matched no valid token shape.
    #[error("invalid token `{text}` at {span}")]
    InvalidToken {
        /// Span of the offending text.
        span: Span,
        /// The offending text.
        text: String,
    },
    /// A `0x` literal had no digits, an odd digit count, or a non-hex digit.
    #[error("invalid hex literal at {span}")]
    InvalidHexLiteral {
        /// Span of the offending literal.
        span: Span,
    },
    /// An integer literal did not fit in an `i64`.
    #[error("integer `{text}` out of range at {span}")]
    IntegerOverflow {
        /// Span of the offending literal.
        span: Span,
        /// The offending text.
        text: String,
    },
    /// A string literal was never closed.
    #[error("unterminated string literal starting at {span}")]
    UnterminatedString {
        /// Span from the opening quote to end of input.
        span: Span,
    },
    /// Input ended while more tokens were required.
    #[error("unexpected end of input, expected {expected}")]
    UnexpectedEof {
        /// A human-readable description of what was expected.
        expected: &'static str,
    },
    /// A token appeared where a different one was required.
    #[error("unexpected {found}, expected {expected} at {span}")]
    UnexpectedToken {
        /// Span of the unexpected token.
        span: Span,
        /// What the parser expected here.
        expected: &'static str,
        /// A description of what was found.
        found: String,
    },
    /// An empty list `()` was used where an application was required.
    #[error("empty application `()` at {span}")]
    EmptyApplication {
        /// Span of the empty list.
        span: Span,
    },
    /// A `defun` appeared somewhere other than top level.
    #[error("`defun` is only allowed at top level, found at {span}")]
    DefunNotAtTopLevel {
        /// Span of the misplaced `defun`.
        span: Span,
    },
    /// An identifier in value position resolved to nothing in a closed scope.
    #[error("undefined identifier `{name}` at {span}")]
    UndefinedIdentifier {
        /// Span of the identifier.
        span: Span,
        /// The identifier text.
        name: String,
    },
    /// An application head named neither a core operator nor a known function.
    #[error("call to unknown function `{name}` at {span}")]
    UnknownFunction {
        /// Span of the head symbol.
        span: Span,
        /// The function name.
        name: String,
    },
    /// A function name was used where a value was expected.
    #[error("function `{name}` used as a value at {span}")]
    FunctionAsValue {
        /// Span of the identifier.
        span: Span,
        /// The function name.
        name: String,
    },
    /// A call or fixed-arity operator received the wrong number of arguments.
    #[error("`{name}` expects {expected} argument(s), got {found} at {span}")]
    ArityMismatch {
        /// Span of the application.
        span: Span,
        /// The callee name.
        name: String,
        /// The expected argument count.
        expected: usize,
        /// The actual argument count.
        found: usize,
    },
    /// Two top-level functions share a name.
    #[error("duplicate function definition `{name}` at {span}")]
    DuplicateFunction {
        /// Span of the redefinition.
        span: Span,
        /// The function name.
        name: String,
    },
    /// A `defun` declared the same parameter name twice.
    #[error("duplicate parameter `{name}` at {span}")]
    DuplicateParameter {
        /// Span of the duplicate parameter.
        span: Span,
        /// The parameter name.
        name: String,
    },
    /// A `let` bound the same name twice.
    #[error("duplicate binding `{name}` at {span}")]
    DuplicateBinding {
        /// Span of the duplicate binding.
        span: Span,
        /// The binding name.
        name: String,
    },
    /// A function (transitively) calls itself; v0,1 requires explicit `a`.
    #[error("recursive call to `{name}` at {span}; use an explicit `a` self-reference in v0,1")]
    RecursiveCall {
        /// Span of the recursive call.
        span: Span,
        /// The function name.
        name: String,
    },
    /// A `let` had more than one body expression (unsupported in v0,1).
    #[error("`let` with multiple body expressions is unsupported in v0,1 at {span}")]
    UnsupportedLetBody {
        /// Span of the `let`.
        span: Span,
    },
    /// A `let`, `if`, or `cond` form appeared inside a quote.
    #[error("cannot quote a `let`/`if`/`cond` form at {span}")]
    UnquotableForm {
        /// Span of the offending form.
        span: Span,
    },
    /// The program contained no top-level expression to evaluate.
    #[error("no top-level program expression found")]
    NoProgramExpression,
    /// The program contained more than one top-level expression.
    #[error("multiple top-level program expressions; expected exactly one at {span}")]
    MultipleProgramExpressions {
        /// Span of the second expression.
        span: Span,
    },
}

impl Error {
    /// The source span this error points at, if any.
    ///
    /// [`Error::UnexpectedEof`] has no span because it refers to the (empty)
    /// position past the end of the source.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            Error::InvalidToken { span, .. }
            | Error::InvalidHexLiteral { span }
            | Error::IntegerOverflow { span, .. }
            | Error::UnterminatedString { span }
            | Error::UnexpectedToken { span, .. }
            | Error::EmptyApplication { span }
            | Error::DefunNotAtTopLevel { span }
            | Error::UndefinedIdentifier { span, .. }
            | Error::UnknownFunction { span, .. }
            | Error::FunctionAsValue { span, .. }
            | Error::ArityMismatch { span, .. }
            | Error::DuplicateFunction { span, .. }
            | Error::DuplicateParameter { span, .. }
            | Error::DuplicateBinding { span, .. }
            | Error::RecursiveCall { span, .. }
            | Error::UnsupportedLetBody { span }
            | Error::UnquotableForm { span }
            | Error::MultipleProgramExpressions { span } => Some(*span),
            Error::UnexpectedEof { .. } | Error::NoProgramExpression => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_is_some_for_located_errors() {
        let e = Error::InvalidHexLiteral {
            span: Span::new(2, 5),
        };
        assert_eq!(e.span(), Some(Span::new(2, 5)));
    }

    #[test]
    fn span_is_none_for_eof() {
        let e = Error::UnexpectedEof {
            expected: "expression",
        };
        assert_eq!(e.span(), None);
    }
}
