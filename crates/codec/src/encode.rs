//! Canonical encoder: [`CoreExpr`] to bytes.
//!
//! The encoding follows spec §7. It is *canonical*: each [`CoreExpr`] maps to
//! exactly one byte string, which is what makes `decode(encode(e)) == e` hold.
//!
//! Documented choices for the under-specified codes:
//! - `0x34`: an atom whose length does not fit the literal/short ranges — a
//!   single length byte follows, then that many payload bytes. Used for
//!   length-1 atoms outside `0x01..=0x33` and for lengths `65..=97`.
//! - `0x74`: an unsigned-LEB128 length (for lengths `>= 98`), then payload.
//! - `0x7f`: an unsigned-LEB128 header `size << 1 | proper_flag`, then `size`
//!   entries (the last is the terminator when `proper_flag == 0`).
//! - `0x80..=0xff`: the quote shorthand for `(q . X)` — encode `X`, then set the
//!   high bit of its first byte. Only applicable when that bit is clear, so a
//!   `(q . (q . Y))` falls back to the generic two-entry list encoding.

use btclisp_core::CoreExpr;

/// Encode a core expression into its canonical byte form.
///
/// # Example
///
/// ```
/// use btclisp_core::CoreExpr;
/// use btclisp_codec::encode;
///
/// assert_eq!(encode(&CoreExpr::Nil), vec![0x00]);
/// assert_eq!(encode(&CoreExpr::atom(vec![0x05])), vec![0x05]);
/// ```
#[must_use]
pub fn encode(expr: &CoreExpr) -> Vec<u8> {
    let mut out = Vec::new();
    encode_into(expr, &mut out);
    out
}

fn encode_into(expr: &CoreExpr, out: &mut Vec<u8>) {
    match expr {
        CoreExpr::Nil => out.push(0x00),
        CoreExpr::Atom(bytes) => encode_atom(bytes, out),
        CoreExpr::Cons(head, tail) => {
            if is_q_atom(head) {
                let mut inner = Vec::new();
                encode_into(tail, &mut inner);
                if inner[0] & 0x80 == 0 {
                    inner[0] |= 0x80;
                    out.extend_from_slice(&inner);
                    return;
                }
            }
            encode_cons(expr, out);
        }
    }
}

fn encode_atom(bytes: &[u8], out: &mut Vec<u8>) {
    let len = bytes.len();
    match len {
        0 => out.push(0x00),
        1 if (0x01..=0x33).contains(&bytes[0]) => out.push(bytes[0]),
        1 => {
            out.push(0x34);
            out.push(1);
            out.push(bytes[0]);
        }
        2..=64 => {
            out.push(0x35 + (len - 2) as u8);
            out.extend_from_slice(bytes);
        }
        65..=97 => {
            out.push(0x34);
            out.push(len as u8);
            out.extend_from_slice(bytes);
        }
        _ => {
            out.push(0x74);
            write_varint(len as u64, out);
            out.extend_from_slice(bytes);
        }
    }
}

/// Encode a cons cell by walking its right spine into a (im)proper list.
fn encode_cons(expr: &CoreExpr, out: &mut Vec<u8>) {
    let mut elems: Vec<&CoreExpr> = Vec::new();
    let mut terminator: Option<&CoreExpr> = None;
    let mut cur = expr;
    loop {
        match cur {
            CoreExpr::Cons(head, tail) => {
                elems.push(head);
                cur = tail;
            }
            CoreExpr::Nil => break,
            atom => {
                terminator = Some(atom);
                break;
            }
        }
    }

    match terminator {
        Some(term) => {
            let entries = elems.len() + 1;
            if (2..=6).contains(&entries) {
                out.push(0x7a + (entries - 2) as u8);
            } else {
                out.push(0x7f);
                write_varint((entries as u64) << 1, out);
            }
            for el in elems {
                encode_into(el, out);
            }
            encode_into(term, out);
        }
        None => {
            let n = elems.len();
            if (1..=5).contains(&n) {
                out.push(0x75 + (n - 1) as u8);
            } else {
                out.push(0x7f);
                write_varint(((n as u64) << 1) | 1, out);
            }
            for el in elems {
                encode_into(el, out);
            }
        }
    }
}

/// Is `expr` the quote opcode atom `0x00`?
fn is_q_atom(expr: &CoreExpr) -> bool {
    matches!(expr, CoreExpr::Atom(bytes) if bytes.len() == 1 && bytes[0] == 0x00)
}

/// Append `value` as an unsigned LEB128 varint.
fn write_varint(mut value: u64, out: &mut Vec<u8>) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_nil_and_literal_atoms() {
        assert_eq!(encode(&CoreExpr::Nil), vec![0x00]);
        assert_eq!(encode(&CoreExpr::atom(vec![0x01])), vec![0x01]);
        assert_eq!(encode(&CoreExpr::atom(vec![0x33])), vec![0x33]);
    }

    #[test]
    fn encodes_leftover_single_byte_atoms() {
        assert_eq!(encode(&CoreExpr::atom(vec![0x00])), vec![0x34, 0x01, 0x00]);
        assert_eq!(encode(&CoreExpr::atom(vec![0x34])), vec![0x34, 0x01, 0x34]);
        assert_eq!(encode(&CoreExpr::atom(vec![0xff])), vec![0x34, 0x01, 0xff]);
    }

    #[test]
    fn encodes_atom_length_boundaries() {
        assert_eq!(encode(&CoreExpr::atom(vec![0xaa; 2]))[0], 0x35);
        assert_eq!(encode(&CoreExpr::atom(vec![0xaa; 64]))[0], 0x73);
        assert_eq!(&encode(&CoreExpr::atom(vec![0xaa; 65]))[..2], &[0x34, 65]);
        assert_eq!(&encode(&CoreExpr::atom(vec![0xaa; 97]))[..2], &[0x34, 97]);
        // length 98 -> 0x74, varint(98) = 0x62
        assert_eq!(&encode(&CoreExpr::atom(vec![0xaa; 98]))[..2], &[0x74, 0x62]);
    }

    #[test]
    fn quotes_with_high_bit() {
        // (q . 0x05) -> 0x85
        let quoted = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x05]));
        assert_eq!(encode(&quoted), vec![0x85]);
        // (q . nil) -> 0x80
        let quoted_nil = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::Nil);
        assert_eq!(encode(&quoted_nil), vec![0x80]);
    }

    #[test]
    fn nested_quote_falls_back_to_generic_list() {
        // (q . (q . nil)) cannot reuse the high bit; encodes as a 2-entry list.
        let inner = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::Nil);
        let outer = CoreExpr::cons(CoreExpr::atom(vec![0x00]), inner);
        assert_eq!(
            encode(&outer),
            vec![0x76, 0x34, 0x01, 0x00, 0x34, 0x01, 0x00]
        );
    }

    #[test]
    fn encodes_application_list() {
        // (+ 2 3) lowered: (0x17 (q.0x02) (q.0x03)) -> proper list of 3.
        let expr = CoreExpr::cons(
            CoreExpr::atom(vec![0x17]),
            CoreExpr::cons(
                CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x02])),
                CoreExpr::cons(
                    CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x03])),
                    CoreExpr::Nil,
                ),
            ),
        );
        assert_eq!(encode(&expr), vec![0x77, 0x17, 0x82, 0x83]);
    }

    #[test]
    fn encodes_improper_pair() {
        let pair = CoreExpr::cons(CoreExpr::atom(vec![0x01]), CoreExpr::atom(vec![0x02]));
        assert_eq!(encode(&pair), vec![0x7a, 0x01, 0x02]);
    }

    #[test]
    fn encodes_long_proper_list_via_0x7f() {
        // six-element proper list -> 0x7f, varint(6<<1 | 1 = 13) = 0x0d
        let mut list = CoreExpr::Nil;
        for _ in 0..6 {
            list = CoreExpr::cons(CoreExpr::atom(vec![0x01]), list);
        }
        let bytes = encode(&list);
        assert_eq!(&bytes[..2], &[0x7f, 0x0d]);
        assert_eq!(bytes.len(), 2 + 6);
    }

    #[test]
    fn varint_is_little_endian_base128() {
        let mut out = Vec::new();
        write_varint(300, &mut out); // 300 = 0b100101100 -> 0xac 0x02
        assert_eq!(out, vec![0xac, 0x02]);
    }
}
