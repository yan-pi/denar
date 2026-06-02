//! Hand-rolled lexer.
//!
//! Tokens are delimited by whitespace, parentheses, the quote character, the
//! comment marker `;`, or string quotes. Each emitted [`Token`] carries the
//! byte span it occupies in the source.

use crate::ast::Span;
use crate::error::Error;

/// A lexical token.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Tok {
    /// `(`
    LParen,
    /// `)`
    RParen,
    /// `'` (quote sugar prefix)
    Quote,
    /// `nil`
    Nil,
    /// A signed decimal integer.
    Int(i64),
    /// A `0x` byte literal.
    Hex(Vec<u8>),
    /// A string literal (already unescaped).
    Str(String),
    /// An identifier or operator symbol.
    Sym(String),
}

/// A token paired with its source span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Token {
    /// The token kind.
    pub tok: Tok,
    /// The span the token occupies.
    pub span: Span,
}

/// Single-character operator symbols that are not valid identifiers.
const PUNCT_OPS: [&str; 7] = ["+", "-", "*", "/", "%", "=", "<"];

/// Lex an entire source string into a token stream.
///
/// # Errors
///
/// Returns an [`Error`] for malformed literals (bad hex, integer overflow,
/// unterminated strings) or unrecognised tokens.
///
/// # Example
///
/// ```
/// use btclisp_compiler::lex::{lex, Tok};
///
/// let toks = lex("(+ 1 2)").unwrap();
/// assert_eq!(toks.first().unwrap().tok, Tok::LParen);
/// ```
pub fn lex(src: &str) -> Result<Vec<Token>, Error> {
    let bytes = src.as_bytes();
    let mut i = 0usize;
    let mut out = Vec::new();

    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b';' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'(' => {
                out.push(token(Tok::LParen, i, i + 1));
                i += 1;
            }
            b')' => {
                out.push(token(Tok::RParen, i, i + 1));
                i += 1;
            }
            b'\'' => {
                out.push(token(Tok::Quote, i, i + 1));
                i += 1;
            }
            b'"' => {
                let (s, end) = lex_string(bytes, i)?;
                out.push(token(Tok::Str(s), i, end));
                i = end;
            }
            _ => {
                let start = i;
                while i < bytes.len() && !is_delimiter(bytes[i]) {
                    i += 1;
                }
                let word = &src[start..i];
                out.push(token(classify(word, Span::new(start, i))?, start, i));
            }
        }
    }

    Ok(out)
}

fn token(tok: Tok, start: usize, end: usize) -> Token {
    Token {
        tok,
        span: Span::new(start, end),
    }
}

/// Returns `true` for bytes that terminate a bare word.
fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'\'' | b'"' | b';'
    )
}

/// Classify a bare word into a token, validating literal shapes.
fn classify(word: &str, span: Span) -> Result<Tok, Error> {
    if word == "nil" {
        return Ok(Tok::Nil);
    }
    if let Some(hex) = word.strip_prefix("0x") {
        return classify_hex(hex, span);
    }
    if is_integer(word) {
        return word
            .parse::<i64>()
            .map(Tok::Int)
            .map_err(|_| Error::IntegerOverflow {
                span,
                text: word.to_string(),
            });
    }
    if is_symbol(word) {
        return Ok(Tok::Sym(word.to_string()));
    }
    Err(Error::InvalidToken {
        span,
        text: word.to_string(),
    })
}

fn classify_hex(hex: &str, span: Span) -> Result<Tok, Error> {
    if hex.is_empty() || hex.len() % 2 != 0 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(Error::InvalidHexLiteral { span });
    }
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|k| u8::from_str_radix(&hex[k..k + 2], 16))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|_| Error::InvalidHexLiteral { span })?;
    Ok(Tok::Hex(bytes))
}

/// Does `w` match `-?[0-9]+`?
fn is_integer(w: &str) -> bool {
    let digits = w.strip_prefix('-').unwrap_or(w);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// Does `w` match the identifier grammar, or a known punctuation operator?
fn is_symbol(w: &str) -> bool {
    if PUNCT_OPS.contains(&w) {
        return true;
    }
    let mut chars = w.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '?' | '!'))
}

/// Scan a string literal beginning at the opening quote `bytes[start]`.
///
/// Returns the unescaped contents and the index one past the closing quote.
fn lex_string(bytes: &[u8], start: usize) -> Result<(String, usize), Error> {
    let mut i = start + 1;
    let mut buf: Vec<u8> = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let s = String::from_utf8(buf).map_err(|_| Error::InvalidToken {
                    span: Span::new(start, i + 1),
                    text: "<non-utf8 string literal>".to_string(),
                })?;
                return Ok((s, i + 1));
            }
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                match bytes[i] {
                    b'"' => buf.push(b'"'),
                    b'\\' => buf.push(b'\\'),
                    b'n' => buf.push(b'\n'),
                    b't' => buf.push(b'\t'),
                    b'r' => buf.push(b'\r'),
                    b'0' => buf.push(0),
                    other => {
                        buf.push(b'\\');
                        buf.push(other);
                    }
                }
                i += 1;
            }
            other => {
                buf.push(other);
                i += 1;
            }
        }
    }
    Err(Error::UnterminatedString {
        span: Span::new(start, bytes.len()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<Tok> {
        lex(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn lexes_application_tokens() {
        assert_eq!(
            kinds("(+ 1 2)"),
            vec![
                Tok::LParen,
                Tok::Sym("+".to_string()),
                Tok::Int(1),
                Tok::Int(2),
                Tok::RParen
            ]
        );
    }

    #[test]
    fn lexes_negative_integer() {
        assert_eq!(kinds("-7"), vec![Tok::Int(-7)]);
    }

    #[test]
    fn lone_minus_is_a_symbol() {
        assert_eq!(kinds("-"), vec![Tok::Sym("-".to_string())]);
    }

    #[test]
    fn lexes_hex_literal() {
        assert_eq!(
            kinds("0xdeadbeef"),
            vec![Tok::Hex(vec![0xde, 0xad, 0xbe, 0xef])]
        );
    }

    #[test]
    fn rejects_odd_length_hex() {
        let err = lex("0xabc").unwrap_err();
        assert!(matches!(err, Error::InvalidHexLiteral { .. }));
    }

    #[test]
    fn rejects_non_hex_digits() {
        let err = lex("0xzz").unwrap_err();
        assert!(matches!(err, Error::InvalidHexLiteral { .. }));
    }

    #[test]
    fn lexes_nil_keyword() {
        assert_eq!(kinds("nil"), vec![Tok::Nil]);
    }

    #[test]
    fn lexes_identifier_with_special_chars() {
        assert_eq!(
            kinds("valid-name?"),
            vec![Tok::Sym("valid-name?".to_string())]
        );
    }

    #[test]
    fn lexes_string_with_escapes() {
        assert_eq!(kinds(r#""a\"b\n""#), vec![Tok::Str("a\"b\n".to_string())]);
    }

    #[test]
    fn rejects_unterminated_string() {
        let err = lex("\"oops").unwrap_err();
        assert!(matches!(err, Error::UnterminatedString { .. }));
    }

    #[test]
    fn skips_line_comments() {
        assert_eq!(kinds("1 ; comment\n2"), vec![Tok::Int(1), Tok::Int(2)]);
    }

    #[test]
    fn lexes_quote_prefix() {
        assert_eq!(kinds("'x"), vec![Tok::Quote, Tok::Sym("x".to_string())]);
    }

    #[test]
    fn rejects_word_starting_with_digit() {
        let err = lex("1abc").unwrap_err();
        assert!(matches!(err, Error::InvalidToken { .. }));
    }

    #[test]
    fn records_spans() {
        let toks = lex("(+ 1)").unwrap();
        assert_eq!(toks[0].span, Span::new(0, 1));
        assert_eq!(toks[1].span, Span::new(1, 2));
        assert_eq!(toks[2].span, Span::new(3, 4));
        assert_eq!(toks[3].span, Span::new(4, 5));
    }
}
