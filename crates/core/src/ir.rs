//! The core intermediate representation: a minimal nil/atom/cons tree.

use bytes::Bytes;

/// A btclisp core expression.
///
/// Every program lowers to this shape: the empty list ([`CoreExpr::Nil`]),
/// a byte-string atom ([`CoreExpr::Atom`]), or a cons cell pairing two
/// sub-expressions ([`CoreExpr::Cons`]).
///
/// # Example
///
/// ```
/// use btclisp_core::CoreExpr;
///
/// let list = CoreExpr::cons(CoreExpr::atom(vec![1]), CoreExpr::Nil);
/// assert!(!list.is_atom());
/// assert!(CoreExpr::Nil.is_atom());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreExpr {
    /// The empty list, also the canonical "false" / empty atom.
    Nil,
    /// A byte-string atom.
    Atom(Bytes),
    /// A cons cell: `(head . tail)`.
    Cons(Box<CoreExpr>, Box<CoreExpr>),
}

impl CoreExpr {
    /// Construct an [`CoreExpr::Atom`] from anything convertible into [`Bytes`].
    #[must_use]
    pub fn atom(bytes: impl Into<Bytes>) -> Self {
        CoreExpr::Atom(bytes.into())
    }

    /// Construct a [`CoreExpr::Cons`] cell from `head` and `tail`.
    #[must_use]
    pub fn cons(head: CoreExpr, tail: CoreExpr) -> Self {
        CoreExpr::Cons(Box::new(head), Box::new(tail))
    }

    /// Returns `true` for atoms, where `nil` counts as the empty atom.
    #[must_use]
    pub fn is_atom(&self) -> bool {
        matches!(self, CoreExpr::Nil | CoreExpr::Atom(_))
    }

    /// Render this expression as an s-expression for debugging and disassembly.
    ///
    /// Atoms are shown as `0x<hex>`, `nil` and the empty list as `()`, and cons
    /// chains in list form, falling back to dotted notation for improper tails.
    ///
    /// # Example
    ///
    /// ```
    /// use btclisp_core::CoreExpr;
    ///
    /// let quoted = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x05]));
    /// assert_eq!(quoted.to_sexpr(), "(0x00 . 0x05)");
    /// ```
    #[must_use]
    pub fn to_sexpr(&self) -> String {
        let mut out = String::new();
        self.write_sexpr(&mut out);
        out
    }

    fn write_sexpr(&self, out: &mut String) {
        use std::fmt::Write as _;
        match self {
            CoreExpr::Nil => out.push_str("()"),
            CoreExpr::Atom(bytes) => {
                out.push_str("0x");
                for byte in bytes {
                    let _ = write!(out, "{byte:02x}");
                }
            }
            CoreExpr::Cons(_, _) => {
                out.push('(');
                let mut cur = self;
                let mut first = true;
                loop {
                    match cur {
                        CoreExpr::Cons(head, tail) => {
                            if !first {
                                out.push(' ');
                            }
                            first = false;
                            head.write_sexpr(out);
                            cur = tail;
                        }
                        CoreExpr::Nil => break,
                        atom => {
                            out.push_str(" . ");
                            atom.write_sexpr(out);
                            break;
                        }
                    }
                }
                out.push(')');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nil_is_an_atom() {
        assert!(CoreExpr::Nil.is_atom());
    }

    #[test]
    fn atom_is_an_atom() {
        assert!(CoreExpr::atom(vec![1, 2, 3]).is_atom());
    }

    #[test]
    fn cons_is_not_an_atom() {
        let cell = CoreExpr::cons(CoreExpr::atom(vec![1]), CoreExpr::Nil);
        assert!(!cell.is_atom());
    }

    #[test]
    fn sexpr_renders_proper_list() {
        let list = CoreExpr::cons(
            CoreExpr::atom(vec![0x17]),
            CoreExpr::cons(CoreExpr::atom(vec![0x02]), CoreExpr::Nil),
        );
        assert_eq!(list.to_sexpr(), "(0x17 0x02)");
    }

    #[test]
    fn sexpr_renders_improper_tail() {
        let dotted = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x05]));
        assert_eq!(dotted.to_sexpr(), "(0x00 . 0x05)");
    }

    #[test]
    fn sexpr_renders_nil_as_empty_list() {
        assert_eq!(CoreExpr::Nil.to_sexpr(), "()");
        let quoted_nil = CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::Nil);
        assert_eq!(quoted_nil.to_sexpr(), "(0x00)");
    }

    #[test]
    fn cons_constructor_nests_boxed_children() {
        let cell = CoreExpr::cons(CoreExpr::atom(vec![0]), CoreExpr::atom(vec![1]));
        let expected = CoreExpr::Cons(
            Box::new(CoreExpr::Atom(Bytes::from(vec![0]))),
            Box::new(CoreExpr::Atom(Bytes::from(vec![1]))),
        );
        assert_eq!(cell, expected);
    }
}
