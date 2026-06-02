//! The v0,1 opcode subset of ajtowns' BTC Lisp table.
//!
//! Byte values are taken verbatim from the reference table; all bytes not
//! listed here are reserved and decode to `None`.

/// A core opcode, tagged with its on-the-wire byte value.
///
/// # Example
///
/// ```
/// use btclisp_core::Opcode;
///
/// assert_eq!(Opcode::from_byte(0x17), Some(Opcode::Add));
/// assert_eq!(Opcode::Add.as_byte(), 0x17);
/// assert_eq!(Opcode::Add.name(), "+");
/// assert_eq!(Opcode::from_byte(0x04), None);
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    /// `q` — quote: return argument unevaluated.
    Q = 0x00,
    /// `a` — apply: evaluate program in a constructed environment.
    A = 0x01,
    /// `x` — exception: fail the program.
    X = 0x02,
    /// `i` — if: strict in the condition only.
    If = 0x03,
    /// `c` — cons.
    C = 0x05,
    /// `h` — head.
    H = 0x06,
    /// `t` — tail.
    T = 0x07,
    /// `l` — list?
    L = 0x08,
    /// `not`.
    Not = 0x09,
    /// `all`.
    All = 0x0a,
    /// `any`.
    Any = 0x0b,
    /// `=` — equality.
    Eq = 0x0c,
    /// `strlen` — sum of argument lengths.
    Strlen = 0x0e,
    /// `substr`.
    Substr = 0x0f,
    /// `cat` — concatenation.
    Cat = 0x10,
    /// `+`.
    Add = 0x17,
    /// `-`.
    Sub = 0x18,
    /// `*`.
    Mul = 0x19,
    /// `%` — modulo.
    Mod = 0x1a,
    /// `/%` — divmod; `/` is exposed as `(h (/% a b))`.
    DivMod = 0x1b,
    /// `<` — unsigned little-endian less-than.
    Lt = 0x1e,
    /// `b` — bintree (balanced cons; v0,2 use).
    BinTree = 0x20,
    /// `rd` — bytes to expression.
    Rd = 0x22,
    /// `wr` — expression to bytes.
    Wr = 0x23,
    /// `sha256`.
    Sha256 = 0x24,
    /// `hash160` — ripemd160(sha256(..)).
    Hash160 = 0x26,
    /// `hash256` — sha256(sha256(..)).
    Hash256 = 0x27,
    /// `bip340_verify`.
    Bip340Verify = 0x28,
    /// `tx` — transaction introspection.
    Tx = 0x2b,
    /// `bip342_txmsg` — BIP341 sighash message.
    Bip342Txmsg = 0x2c,
}

impl Opcode {
    /// Decode a byte into an [`Opcode`], or `None` if the byte is reserved.
    #[must_use]
    pub fn from_byte(b: u8) -> Option<Self> {
        let op = match b {
            0x00 => Opcode::Q,
            0x01 => Opcode::A,
            0x02 => Opcode::X,
            0x03 => Opcode::If,
            0x05 => Opcode::C,
            0x06 => Opcode::H,
            0x07 => Opcode::T,
            0x08 => Opcode::L,
            0x09 => Opcode::Not,
            0x0a => Opcode::All,
            0x0b => Opcode::Any,
            0x0c => Opcode::Eq,
            0x0e => Opcode::Strlen,
            0x0f => Opcode::Substr,
            0x10 => Opcode::Cat,
            0x17 => Opcode::Add,
            0x18 => Opcode::Sub,
            0x19 => Opcode::Mul,
            0x1a => Opcode::Mod,
            0x1b => Opcode::DivMod,
            0x1e => Opcode::Lt,
            0x20 => Opcode::BinTree,
            0x22 => Opcode::Rd,
            0x23 => Opcode::Wr,
            0x24 => Opcode::Sha256,
            0x26 => Opcode::Hash160,
            0x27 => Opcode::Hash256,
            0x28 => Opcode::Bip340Verify,
            0x2b => Opcode::Tx,
            0x2c => Opcode::Bip342Txmsg,
            _ => return None,
        };
        Some(op)
    }

    /// The on-the-wire byte value of this opcode.
    #[must_use]
    pub fn as_byte(self) -> u8 {
        self as u8
    }

    /// The canonical core-language name of this opcode.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Opcode::Q => "q",
            Opcode::A => "a",
            Opcode::X => "x",
            Opcode::If => "i",
            Opcode::C => "c",
            Opcode::H => "h",
            Opcode::T => "t",
            Opcode::L => "l",
            Opcode::Not => "not",
            Opcode::All => "all",
            Opcode::Any => "any",
            Opcode::Eq => "=",
            Opcode::Strlen => "strlen",
            Opcode::Substr => "substr",
            Opcode::Cat => "cat",
            Opcode::Add => "+",
            Opcode::Sub => "-",
            Opcode::Mul => "*",
            Opcode::Mod => "%",
            Opcode::DivMod => "/%",
            Opcode::Lt => "<",
            Opcode::BinTree => "b",
            Opcode::Rd => "rd",
            Opcode::Wr => "wr",
            Opcode::Sha256 => "sha256",
            Opcode::Hash160 => "hash160",
            Opcode::Hash256 => "hash256",
            Opcode::Bip340Verify => "bip340_verify",
            Opcode::Tx => "tx",
            Opcode::Bip342Txmsg => "bip342_txmsg",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every opcode in the table, used to drive exhaustive round-trip tests.
    const ALL: &[Opcode] = &[
        Opcode::Q,
        Opcode::A,
        Opcode::X,
        Opcode::If,
        Opcode::C,
        Opcode::H,
        Opcode::T,
        Opcode::L,
        Opcode::Not,
        Opcode::All,
        Opcode::Any,
        Opcode::Eq,
        Opcode::Strlen,
        Opcode::Substr,
        Opcode::Cat,
        Opcode::Add,
        Opcode::Sub,
        Opcode::Mul,
        Opcode::Mod,
        Opcode::DivMod,
        Opcode::Lt,
        Opcode::BinTree,
        Opcode::Rd,
        Opcode::Wr,
        Opcode::Sha256,
        Opcode::Hash160,
        Opcode::Hash256,
        Opcode::Bip340Verify,
        Opcode::Tx,
        Opcode::Bip342Txmsg,
    ];

    #[test]
    fn byte_round_trips_for_every_opcode() {
        for &op in ALL {
            assert_eq!(Opcode::from_byte(op.as_byte()), Some(op));
        }
    }

    #[test]
    fn names_are_unique() {
        let mut names: Vec<&str> = ALL.iter().map(|op| op.name()).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total, "opcode names must be unique");
    }

    #[test]
    fn reserved_bytes_decode_to_none() {
        assert_eq!(Opcode::from_byte(0x04), None);
        assert_eq!(Opcode::from_byte(0x0d), None);
        assert_eq!(Opcode::from_byte(0xff), None);
    }

    #[test]
    fn known_byte_values_match_spec() {
        assert_eq!(Opcode::Q.as_byte(), 0x00);
        assert_eq!(Opcode::Bip342Txmsg.as_byte(), 0x2c);
        assert_eq!(Opcode::Lt.as_byte(), 0x1e);
    }
}
