//! The single public error type for evaluation.

/// An error raised while evaluating a core expression.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EvalError {
    /// The `x` opcode (or an internal failure) aborted the program.
    #[error("program raised an exception")]
    Exception,
    /// An operator position held something other than a known opcode atom.
    #[error("invalid operator: {0:02x?}")]
    InvalidOperator(Vec<u8>),
    /// An environment path walked into an atom instead of a cons.
    #[error("environment path led into an atom")]
    PathIntoAtom,
    /// `h` was applied to a non-cons value.
    #[error("cannot take the head of an atom")]
    HeadOfAtom,
    /// `t` was applied to a non-cons value.
    #[error("cannot take the tail of an atom")]
    TailOfAtom,
    /// An argument list was improper (not nil-terminated).
    #[error("improper argument list")]
    ImproperArgList,
    /// An opcode received the wrong number of arguments.
    #[error("`{op}` expects {expected} argument(s), got {found}")]
    Arity {
        /// The opcode name.
        op: &'static str,
        /// Expected argument count.
        expected: usize,
        /// Actual argument count.
        found: usize,
    },
    /// An argument had the wrong shape (e.g. a list where an atom was needed).
    #[error("type error: {0}")]
    TypeError(&'static str),
    /// Division or modulo by zero.
    #[error("division by zero")]
    DivByZero,
    /// An opcode is recognised but not yet implemented in this sprint.
    #[error("`{0}` is not implemented yet")]
    NotImplemented(&'static str),
    /// A `tx`/`bip342_txmsg` opcode ran with no transaction context.
    #[error("no transaction context available")]
    NoTxContext,
    /// A `tx` introspection code did not name a known field.
    #[error("unknown tx field code {0}")]
    UnknownTxField(u64),
    /// The current input index was out of range for the transaction/prevouts.
    #[error("input index out of range")]
    InputIndexOutOfRange,
    /// Computing the BIP341 sighash failed.
    #[error("sighash error: {0}")]
    SighashError(String),
    /// A `bip340_verify` argument (key/message/signature) was malformed.
    #[error("invalid schnorr input: {0}")]
    SchnorrInput(&'static str),
    /// A `bip340_verify` signature was well-formed but did not verify.
    #[error("signature verification failed")]
    SignatureVerifyFailed,
    /// `rd` failed to decode its argument.
    #[error("decode error: {0}")]
    Decode(#[from] btclisp_codec::Error),
    /// The operation-count budget was exhausted.
    #[error("operation limit exceeded")]
    OpLimitExceeded,
    /// The call-depth budget was exhausted.
    #[error("stack depth limit exceeded")]
    StackLimitExceeded,
}
