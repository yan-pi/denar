//! Surface abstract syntax tree, with source spans for diagnostics.
//!
//! Every node carries a [`Span`] (byte offsets into the original source).
//! The [`std::fmt::Display`] impls render the canonical, re-parseable form of
//! each node — this is the basis of the `fmt` subcommand and the AST
//! pretty-printer.

use std::fmt::{self, Write};

/// A half-open byte range `[start, end)` into the source text.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Span {
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character.
    pub end: usize,
}

impl Span {
    /// Construct a span from a start and end byte offset.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// A leaf value in the surface grammar.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Atom {
    /// A signed decimal integer literal.
    Int(i64),
    /// A `0x`-prefixed raw byte literal.
    Hex(Vec<u8>),
    /// A UTF-8 string literal (bytes = its UTF-8 encoding).
    Str(String),
    /// An identifier: a variable, function name, or core operator.
    Ident(String),
    /// The `nil` literal.
    Nil,
}

/// An identifier occurrence with its source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    /// The identifier text.
    pub name: String,
    /// The span of the identifier.
    pub span: Span,
}

/// A single `let` binding: `(name value)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    /// The bound name.
    pub name: Symbol,
    /// The bound value expression.
    pub value: Expr,
    /// The span covering the whole binding form.
    pub span: Span,
}

/// A single `cond` clause: `(test result)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clause {
    /// The test expression.
    pub test: Expr,
    /// The result expression, evaluated when `test` is truthy.
    pub result: Expr,
    /// The span covering the whole clause.
    pub span: Span,
}

/// A surface expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    /// A leaf atom.
    Atom {
        /// The atom value.
        atom: Atom,
        /// Source span.
        span: Span,
    },
    /// A quoted expression: `'expr`.
    Quote {
        /// The quoted sub-expression.
        inner: Box<Expr>,
        /// Source span, including the leading quote.
        span: Span,
    },
    /// A `let` form: `(let ((x e) ...) body+)`.
    Let {
        /// One or more bindings.
        bindings: Vec<Binding>,
        /// One or more body expressions.
        body: Vec<Expr>,
        /// Source span.
        span: Span,
    },
    /// An `if` form: `(if cond then else)`.
    If {
        /// The condition.
        cond: Box<Expr>,
        /// The consequent.
        then: Box<Expr>,
        /// The alternative.
        els: Box<Expr>,
        /// Source span.
        span: Span,
    },
    /// A `cond` form: `(cond clause+)`.
    Cond {
        /// One or more clauses.
        clauses: Vec<Clause>,
        /// Source span.
        span: Span,
    },
    /// An application: `(head arg*)`.
    App {
        /// The operator or function name.
        head: Symbol,
        /// Zero or more argument expressions.
        args: Vec<Expr>,
        /// Source span.
        span: Span,
    },
}

impl Expr {
    /// The source span covering this expression.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Expr::Atom { span, .. }
            | Expr::Quote { span, .. }
            | Expr::Let { span, .. }
            | Expr::If { span, .. }
            | Expr::Cond { span, .. }
            | Expr::App { span, .. } => *span,
        }
    }
}

/// A top-level function definition: `(defun name (params*) body)`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Defun {
    /// The function name.
    pub name: Symbol,
    /// The formal parameters.
    pub params: Vec<Symbol>,
    /// The function body.
    pub body: Expr,
    /// Source span.
    pub span: Span,
}

/// A top-level program element: a `defun` or a bare expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Form {
    /// A function definition.
    Defun(Defun),
    /// A bare expression.
    Expr(Expr),
}

impl fmt::Display for Atom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Atom::Int(n) => write!(f, "{n}"),
            Atom::Hex(bytes) => {
                f.write_str("0x")?;
                for b in bytes {
                    write!(f, "{b:02x}")?;
                }
                Ok(())
            }
            Atom::Str(s) => write_escaped_str(f, s),
            Atom::Ident(s) => f.write_str(s),
            Atom::Nil => f.write_str("nil"),
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Atom { atom, .. } => write!(f, "{atom}"),
            Expr::Quote { inner, .. } => write!(f, "'{inner}"),
            Expr::App { head, args, .. } => {
                write!(f, "({head}")?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                f.write_char(')')
            }
            Expr::If {
                cond, then, els, ..
            } => write!(f, "(if {cond} {then} {els})"),
            Expr::Let { bindings, body, .. } => {
                f.write_str("(let (")?;
                for (i, b) in bindings.iter().enumerate() {
                    if i > 0 {
                        f.write_char(' ')?;
                    }
                    write!(f, "({} {})", b.name, b.value)?;
                }
                f.write_char(')')?;
                for e in body {
                    write!(f, " {e}")?;
                }
                f.write_char(')')
            }
            Expr::Cond { clauses, .. } => {
                f.write_str("(cond")?;
                for c in clauses {
                    write!(f, " ({} {})", c.test, c.result)?;
                }
                f.write_char(')')
            }
        }
    }
}

impl fmt::Display for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Form::Defun(d) => {
                write!(f, "(defun {} (", d.name)?;
                for (i, p) in d.params.iter().enumerate() {
                    if i > 0 {
                        f.write_char(' ')?;
                    }
                    write!(f, "{p}")?;
                }
                write!(f, ") {})", d.body)
            }
            Form::Expr(e) => write!(f, "{e}"),
        }
    }
}

/// Render a string literal with the minimal set of escapes the lexer accepts.
fn write_escaped_str(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_char('"')?;
    for ch in s.chars() {
        match ch {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\t' => f.write_str("\\t")?,
            '\r' => f.write_str("\\r")?,
            c => f.write_char(c)?,
        }
    }
    f.write_char('"')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn displays_integer_atom() {
        let e = Expr::Atom {
            atom: Atom::Int(-42),
            span: span(),
        };
        assert_eq!(e.to_string(), "-42");
    }

    #[test]
    fn displays_hex_atom_lowercase_padded() {
        let a = Atom::Hex(vec![0x0a, 0xff, 0x00]);
        assert_eq!(a.to_string(), "0x0aff00");
    }

    #[test]
    fn displays_string_atom_with_escapes() {
        let a = Atom::Str("a\"b\nc".to_string());
        assert_eq!(a.to_string(), "\"a\\\"b\\nc\"");
    }

    #[test]
    fn displays_application() {
        let e = Expr::App {
            head: Symbol {
                name: "+".to_string(),
                span: span(),
            },
            args: vec![
                Expr::Atom {
                    atom: Atom::Int(1),
                    span: span(),
                },
                Expr::Atom {
                    atom: Atom::Int(2),
                    span: span(),
                },
            ],
            span: span(),
        };
        assert_eq!(e.to_string(), "(+ 1 2)");
    }

    #[test]
    fn span_accessor_returns_node_span() {
        let e = Expr::Atom {
            atom: Atom::Nil,
            span: Span::new(3, 6),
        };
        assert_eq!(e.span(), Span::new(3, 6));
    }
}
