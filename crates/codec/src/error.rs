//! The single public error type for the codec.

/// An error produced while decoding bytes into a [`btclisp_core::CoreExpr`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    /// The input ended before a complete value could be read.
    #[error("unexpected end of input while decoding")]
    UnexpectedEof,
    /// Decoding finished with bytes left over.
    #[error("{0} trailing byte(s) after a complete value")]
    TrailingBytes(usize),
    /// A varint was longer than a `u64` can hold.
    #[error("varint exceeds 64 bits")]
    VarintTooLong,
    /// A list header declared fewer entries than its kind allows.
    #[error("malformed list: improper list needs at least two entries")]
    MalformedList,
}
