//! Human-readable disassembly of core programs.
//!
//! Unlike [`btclisp_core::CoreExpr::to_sexpr`] (which shows raw bytes), this
//! renders operators in head position as opcode mnemonics, quoted values as
//! `(q . <data>)`, and bare atoms (environment references) as their integer
//! path — i.e. it reads a `CoreExpr` *as a program*.

use btclisp_core::{CoreExpr, Opcode};

/// Disassemble a core program into a mnemonic s-expression string.
///
/// # Example
///
/// ```
/// use btclisp_core::CoreExpr;
/// use btclisp_codec::disassemble;
///
/// // (+ . ((q . 2) (q . 3)))  ->  "(+ (q . 0x02) (q . 0x03))"
/// let prog = CoreExpr::cons(
///     CoreExpr::atom(vec![0x17]),
///     CoreExpr::cons(
///         CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x02])),
///         CoreExpr::cons(
///             CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x03])),
///             CoreExpr::Nil,
///         ),
///     ),
/// );
/// assert_eq!(disassemble(&prog), "(+ (q . 0x02) (q . 0x03))");
/// ```
#[must_use]
pub fn disassemble(expr: &CoreExpr) -> String {
    let mut out = String::new();
    write_program(expr, &mut out);
    out
}

fn write_program(expr: &CoreExpr, out: &mut String) {
    match expr {
        CoreExpr::Nil => out.push_str("()"),
        CoreExpr::Atom(bytes) => out.push_str(&env_ref(bytes)),
        CoreExpr::Cons(head, tail) => {
            if is_quote(head) {
                // `(q . X)` quotes literal data; render the tail as data.
                out.push_str("(q . ");
                out.push_str(&tail.to_sexpr());
                out.push(')');
            } else {
                out.push('(');
                write_head(head, out);
                let mut cur = tail.as_ref();
                loop {
                    match cur {
                        CoreExpr::Cons(h, t) => {
                            out.push(' ');
                            write_program(h, out);
                            cur = t;
                        }
                        CoreExpr::Nil => break,
                        atom => {
                            out.push_str(" . ");
                            write_program(atom, out);
                            break;
                        }
                    }
                }
                out.push(')');
            }
        }
    }
}

fn write_head(head: &CoreExpr, out: &mut String) {
    if let CoreExpr::Atom(bytes) = head {
        if bytes.len() == 1 {
            if let Some(op) = Opcode::from_byte(bytes[0]) {
                out.push_str(op.name());
                return;
            }
        }
    }
    write_program(head, out);
}

fn is_quote(expr: &CoreExpr) -> bool {
    matches!(expr, CoreExpr::Atom(bytes) if bytes.as_ref() == [Opcode::Q.as_byte()])
}

/// Render an environment-reference atom as its little-endian integer path.
fn env_ref(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "0".to_string();
    }
    if bytes.len() > 8 {
        let mut hex = String::from("0x");
        use std::fmt::Write as _;
        for b in bytes {
            let _ = write!(hex, "{b:02x}");
        }
        return hex;
    }
    let mut value = 0u64;
    for (i, b) in bytes.iter().enumerate() {
        value |= u64::from(*b) << (8 * i);
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_operator_and_quoted_literals() {
        let prog = CoreExpr::cons(
            CoreExpr::atom(vec![0x17]),
            CoreExpr::cons(
                CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x02])),
                CoreExpr::cons(
                    CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x03])),
                    CoreExpr::Nil,
                ),
            ),
        );
        assert_eq!(disassemble(&prog), "(+ (q . 0x02) (q . 0x03))");
    }

    #[test]
    fn renders_env_references_as_integers() {
        // (+ 2 5) where 2 and 5 are environment paths.
        let prog = CoreExpr::cons(
            CoreExpr::atom(vec![0x17]),
            CoreExpr::cons(
                CoreExpr::atom(vec![0x02]),
                CoreExpr::cons(CoreExpr::atom(vec![0x05]), CoreExpr::Nil),
            ),
        );
        assert_eq!(disassemble(&prog), "(+ 2 5)");
    }

    #[test]
    fn renders_quoted_list_data() {
        let prog = CoreExpr::cons(
            CoreExpr::atom(vec![0x00]),
            CoreExpr::cons(CoreExpr::atom(vec![0x01]), CoreExpr::Nil),
        );
        assert_eq!(disassemble(&prog), "(q . (0x01))");
    }
}
