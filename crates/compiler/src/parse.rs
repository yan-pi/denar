//! Hand-rolled recursive-descent parser over the token stream.

use crate::ast::{Atom, Binding, Clause, Defun, Expr, Form, Span, Symbol};
use crate::error::Error;
use crate::lex::{Tok, Token};

/// A cursor over a token stream that produces surface [`Form`]s.
pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    /// Create a parser over a token stream.
    #[must_use]
    pub fn new(toks: Vec<Token>) -> Self {
        Self { toks, pos: 0 }
    }

    /// Parse the whole token stream into a list of top-level forms.
    ///
    /// # Errors
    ///
    /// Returns the first [`Error`] encountered.
    pub fn parse_program(mut self) -> Result<Vec<Form>, Error> {
        let mut forms = Vec::new();
        while self.pos < self.toks.len() {
            forms.push(self.parse_form()?);
        }
        Ok(forms)
    }

    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }

    fn peek_tok(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    fn peek_tok_at(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n).map(|t| &t.tok)
    }

    fn at_rparen(&self) -> bool {
        matches!(self.peek_tok(), Some(Tok::RParen))
    }

    fn parse_form(&mut self) -> Result<Form, Error> {
        if self.looks_like_defun() {
            Ok(Form::Defun(self.parse_defun()?))
        } else {
            Ok(Form::Expr(self.parse_expr()?))
        }
    }

    fn looks_like_defun(&self) -> bool {
        matches!(self.peek_tok(), Some(Tok::LParen))
            && matches!(self.peek_tok_at(1), Some(Tok::Sym(s)) if s == "defun")
    }

    fn expect(&mut self, want: &Tok, label: &'static str) -> Result<Token, Error> {
        match self.toks.get(self.pos) {
            Some(tk) if tk.tok == *want => {
                let tk = tk.clone();
                self.pos += 1;
                Ok(tk)
            }
            Some(tk) => Err(Error::UnexpectedToken {
                span: tk.span,
                expected: label,
                found: describe(&tk.tok),
            }),
            None => Err(Error::UnexpectedEof { expected: label }),
        }
    }

    fn expect_keyword(&mut self, kw: &'static str) -> Result<(), Error> {
        match self.toks.get(self.pos) {
            Some(tk) => match &tk.tok {
                Tok::Sym(s) if s == kw => {
                    self.pos += 1;
                    Ok(())
                }
                other => Err(Error::UnexpectedToken {
                    span: tk.span,
                    expected: kw,
                    found: describe(other),
                }),
            },
            None => Err(Error::UnexpectedEof { expected: kw }),
        }
    }

    fn expect_symbol(&mut self, label: &'static str) -> Result<Symbol, Error> {
        match self.toks.get(self.pos) {
            Some(tk) => match &tk.tok {
                Tok::Sym(name) => {
                    let sym = Symbol {
                        name: name.clone(),
                        span: tk.span,
                    };
                    self.pos += 1;
                    Ok(sym)
                }
                other => Err(Error::UnexpectedToken {
                    span: tk.span,
                    expected: label,
                    found: describe(other),
                }),
            },
            None => Err(Error::UnexpectedEof { expected: label }),
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, Error> {
        let tk = match self.toks.get(self.pos) {
            Some(t) => t.clone(),
            None => {
                return Err(Error::UnexpectedEof {
                    expected: "expression",
                })
            }
        };
        match tk.tok {
            Tok::Quote => {
                self.pos += 1;
                let inner = self.parse_expr()?;
                let span = Span::new(tk.span.start, inner.span().end);
                Ok(Expr::Quote {
                    inner: Box::new(inner),
                    span,
                })
            }
            Tok::Nil => {
                self.pos += 1;
                Ok(atom_expr(Atom::Nil, tk.span))
            }
            Tok::Int(n) => {
                self.pos += 1;
                Ok(atom_expr(Atom::Int(n), tk.span))
            }
            Tok::Hex(b) => {
                self.pos += 1;
                Ok(atom_expr(Atom::Hex(b), tk.span))
            }
            Tok::Str(s) => {
                self.pos += 1;
                Ok(atom_expr(Atom::Str(s), tk.span))
            }
            Tok::Sym(s) => {
                self.pos += 1;
                Ok(atom_expr(Atom::Ident(s), tk.span))
            }
            Tok::LParen => self.parse_list(tk.span),
            Tok::RParen => Err(Error::UnexpectedToken {
                span: tk.span,
                expected: "expression",
                found: describe(&Tok::RParen),
            }),
        }
    }

    /// Parse a parenthesised form; `open` is the span of the opening `(`.
    fn parse_list(&mut self, open: Span) -> Result<Expr, Error> {
        self.pos += 1; // consume '('
        let head = match self.toks.get(self.pos) {
            Some(tk) => tk.clone(),
            None => {
                return Err(Error::UnexpectedEof {
                    expected: "operator or function name",
                })
            }
        };
        match &head.tok {
            Tok::Sym(s) => match s.as_str() {
                "let" => self.parse_let(open),
                "if" => self.parse_if(open),
                "cond" => self.parse_cond(open),
                "defun" => Err(Error::DefunNotAtTopLevel { span: head.span }),
                _ => self.parse_app(open),
            },
            Tok::RParen => Err(Error::EmptyApplication {
                span: Span::new(open.start, head.span.end),
            }),
            other => Err(Error::UnexpectedToken {
                span: head.span,
                expected: "operator or function name",
                found: describe(other),
            }),
        }
    }

    fn parse_defun(&mut self) -> Result<Defun, Error> {
        let open = self.expect(&Tok::LParen, "(")?;
        self.expect_keyword("defun")?;
        let name = self.expect_symbol("function name")?;
        self.expect(&Tok::LParen, "(")?;
        let mut params = Vec::new();
        while !self.at_rparen() {
            if self.peek_tok().is_none() {
                return Err(Error::UnexpectedEof { expected: ")" });
            }
            params.push(self.expect_symbol("parameter name")?);
        }
        self.expect(&Tok::RParen, ")")?;
        let body = self.parse_expr()?;
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Defun {
            name,
            params,
            body,
            span: Span::new(open.span.start, close.span.end),
        })
    }

    fn parse_app(&mut self, open: Span) -> Result<Expr, Error> {
        let head = self.expect_symbol("operator or function name")?;
        let mut args = Vec::new();
        while !self.at_rparen() {
            if self.peek_tok().is_none() {
                return Err(Error::UnexpectedEof { expected: ")" });
            }
            args.push(self.parse_expr()?);
        }
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Expr::App {
            head,
            args,
            span: Span::new(open.start, close.span.end),
        })
    }

    fn parse_let(&mut self, open: Span) -> Result<Expr, Error> {
        self.pos += 1; // consume 'let'
        self.expect(&Tok::LParen, "(")?;
        let mut bindings = Vec::new();
        while matches!(self.peek_tok(), Some(Tok::LParen)) {
            bindings.push(self.parse_binding()?);
        }
        if bindings.is_empty() {
            return Err(self.unexpected("at least one let binding"));
        }
        self.expect(&Tok::RParen, ")")?;
        let mut body = Vec::new();
        while !self.at_rparen() {
            if self.peek_tok().is_none() {
                return Err(Error::UnexpectedEof { expected: ")" });
            }
            body.push(self.parse_expr()?);
        }
        if body.is_empty() {
            return Err(self.unexpected("at least one let body expression"));
        }
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Expr::Let {
            bindings,
            body,
            span: Span::new(open.start, close.span.end),
        })
    }

    fn parse_binding(&mut self) -> Result<Binding, Error> {
        let open = self.expect(&Tok::LParen, "(")?;
        let name = self.expect_symbol("binding name")?;
        let value = self.parse_expr()?;
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Binding {
            name,
            value,
            span: Span::new(open.span.start, close.span.end),
        })
    }

    fn parse_if(&mut self, open: Span) -> Result<Expr, Error> {
        self.pos += 1; // consume 'if'
        let cond = self.parse_expr()?;
        let then = self.parse_expr()?;
        let els = self.parse_expr()?;
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Expr::If {
            cond: Box::new(cond),
            then: Box::new(then),
            els: Box::new(els),
            span: Span::new(open.start, close.span.end),
        })
    }

    fn parse_cond(&mut self, open: Span) -> Result<Expr, Error> {
        self.pos += 1; // consume 'cond'
        let mut clauses = Vec::new();
        while matches!(self.peek_tok(), Some(Tok::LParen)) {
            clauses.push(self.parse_clause()?);
        }
        if clauses.is_empty() {
            return Err(self.unexpected("at least one cond clause"));
        }
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Expr::Cond {
            clauses,
            span: Span::new(open.start, close.span.end),
        })
    }

    fn parse_clause(&mut self) -> Result<Clause, Error> {
        let open = self.expect(&Tok::LParen, "(")?;
        let test = self.parse_expr()?;
        let result = self.parse_expr()?;
        let close = self.expect(&Tok::RParen, ")")?;
        Ok(Clause {
            test,
            result,
            span: Span::new(open.span.start, close.span.end),
        })
    }

    /// Build an [`Error::UnexpectedToken`]/[`Error::UnexpectedEof`] at the
    /// current position with the given expectation label.
    fn unexpected(&self, expected: &'static str) -> Error {
        match self.peek() {
            Some(tk) => Error::UnexpectedToken {
                span: tk.span,
                expected,
                found: describe(&tk.tok),
            },
            None => Error::UnexpectedEof { expected },
        }
    }
}

fn atom_expr(atom: Atom, span: Span) -> Expr {
    Expr::Atom { atom, span }
}

/// A short human-readable description of a token, for error messages.
fn describe(t: &Tok) -> String {
    match t {
        Tok::LParen => "`(`".to_string(),
        Tok::RParen => "`)`".to_string(),
        Tok::Quote => "`'`".to_string(),
        Tok::Nil => "`nil`".to_string(),
        Tok::Int(n) => format!("integer `{n}`"),
        Tok::Hex(b) => format!("hex literal ({} bytes)", b.len()),
        Tok::Str(_) => "string literal".to_string(),
        Tok::Sym(s) => format!("`{s}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_program;

    /// Parse, then render the canonical form of every top-level form.
    fn fmt(src: &str) -> String {
        parse_program(src)
            .unwrap()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn parses_application_shape() {
        let forms = parse_program("(+ 1 2)").unwrap();
        match &forms[0] {
            Form::Expr(Expr::App { head, args, .. }) => {
                assert_eq!(head.name, "+");
                assert_eq!(args.len(), 2);
            }
            other => panic!("expected application, got {other:?}"),
        }
    }

    #[test]
    fn normalises_whitespace() {
        assert_eq!(fmt("(  +   1\n  2 )"), "(+ 1 2)");
    }

    #[test]
    fn parses_nested_application() {
        assert_eq!(fmt("(+ (* 2 3) 4)"), "(+ (* 2 3) 4)");
    }

    #[test]
    fn parses_defun() {
        assert_eq!(
            fmt("(defun add (a b) (+ a b))"),
            "(defun add (a b) (+ a b))"
        );
    }

    #[test]
    fn parses_let() {
        assert_eq!(
            fmt("(let ((x 1) (y 2)) (+ x y))"),
            "(let ((x 1) (y 2)) (+ x y))"
        );
    }

    #[test]
    fn parses_if() {
        assert_eq!(fmt("(if (< a b) 1 0)"), "(if (< a b) 1 0)");
    }

    #[test]
    fn parses_cond() {
        assert_eq!(
            fmt("(cond ((< a 1) 0) (nil 2))"),
            "(cond ((< a 1) 0) (nil 2))"
        );
    }

    #[test]
    fn parses_quote_sugar() {
        assert_eq!(fmt("'(+ 1 2)"), "'(+ 1 2)");
    }

    #[test]
    fn pretty_print_is_idempotent() {
        let once = fmt("(let ((x 0x0a)) (if (= x nil) \"a\" 'b))");
        let twice = fmt(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn rejects_empty_list() {
        let err = parse_program("()").unwrap_err();
        assert!(matches!(err, Error::EmptyApplication { .. }));
    }

    #[test]
    fn rejects_defun_in_expression_position() {
        let err = parse_program("(+ (defun f () 1))").unwrap_err();
        assert!(matches!(err, Error::DefunNotAtTopLevel { .. }));
    }

    #[test]
    fn rejects_unclosed_application() {
        let err = parse_program("(+ 1 2").unwrap_err();
        assert!(matches!(err, Error::UnexpectedEof { .. }));
    }

    #[test]
    fn rejects_stray_rparen() {
        let err = parse_program(")").unwrap_err();
        assert!(matches!(err, Error::UnexpectedToken { .. }));
    }

    #[test]
    fn parses_multiple_top_level_forms() {
        let forms = parse_program("(defun f (x) x)\n(f 1)").unwrap();
        assert_eq!(forms.len(), 2);
    }
}
