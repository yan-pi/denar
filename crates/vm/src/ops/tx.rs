//! Transaction-introspection opcodes: `tx`, `bip342_txmsg`.
//!
//! These require a [`TxContext`] (set on [`crate::EvalCtx`]); with none, they
//! report [`EvalError::NoTxContext`] (spec §14.2).
//!
//! # `(tx N)` field codes
//!
//! The mapping below is a documented v0,1 choice (to be reconciled with
//! ajtowns' exact table later). Integers return minimal little-endian atoms;
//! hashes, keys, and scripts return byte atoms.
//!
//! | N | field | N | field |
//! |---|-------|---|-------|
//! | 0 | nVersion | 1 | nLockTime |
//! | 2 | input count | 3 | output count |
//! | 4 | current input index | 5 | current input nSequence |
//! | 6 | current input prevout txid | 7 | current input prevout vout |
//! | 8 | current input prevout amount | 9 | current input prevout scriptPubKey |
//! | 10 | tapleaf hash | 11 | internal key (x-only) |
//!
//! `txid` and `tapleaf hash` are returned in internal (non-reversed) byte order.

use bitcoin::hashes::Hash as _;
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::{TapLeafHash, Transaction, TxIn, TxOut, XOnlyPublicKey};

use crate::error::EvalError;
use crate::ops::expect_arity;
use crate::value::Value;

/// The transaction-introspection context for the input being validated (§8).
#[derive(Clone, Debug)]
pub struct TxContext<'a> {
    /// The spending transaction.
    pub tx: &'a Transaction,
    /// Index of the input being validated.
    pub input_index: u32,
    /// The prevouts for every input, indexed identically to `tx.input`.
    pub prevouts: &'a [TxOut],
    /// The tapleaf hash of the executing script.
    pub tapleaf_hash: TapLeafHash,
    /// The taproot internal key.
    pub internal_key: XOnlyPublicKey,
    /// The merkle branch proving the leaf's inclusion (unused in v0,1 sighash).
    pub merkle_branch: Vec<[u8; 32]>,
}

impl TxContext<'_> {
    fn current_input(&self) -> Result<&TxIn, EvalError> {
        self.tx
            .input
            .get(self.input_index as usize)
            .ok_or(EvalError::InputIndexOutOfRange)
    }

    fn current_prevout(&self) -> Result<&TxOut, EvalError> {
        self.prevouts
            .get(self.input_index as usize)
            .ok_or(EvalError::InputIndexOutOfRange)
    }
}

/// `(tx N)` — read a transaction field by code (see module docs).
pub(crate) fn tx(args: &[Value], ctx: Option<&TxContext>) -> Result<Value, EvalError> {
    expect_arity("tx", args, 1)?;
    let ctx = ctx.ok_or(EvalError::NoTxContext)?;
    let code = args[0].as_u64()?;
    let txn = ctx.tx;

    let value = match code {
        0 => Value::from_u64(u64::from(txn.version.0 as u32)),
        1 => Value::from_u64(u64::from(txn.lock_time.to_consensus_u32())),
        2 => Value::from_u64(txn.input.len() as u64),
        3 => Value::from_u64(txn.output.len() as u64),
        4 => Value::from_u64(u64::from(ctx.input_index)),
        5 => Value::from_u64(u64::from(ctx.current_input()?.sequence.to_consensus_u32())),
        6 => byte_atom(&ctx.current_input()?.previous_output.txid.to_byte_array()),
        7 => Value::from_u64(u64::from(ctx.current_input()?.previous_output.vout)),
        8 => Value::from_u64(ctx.current_prevout()?.value.to_sat()),
        9 => byte_atom(ctx.current_prevout()?.script_pubkey.as_bytes()),
        10 => byte_atom(&ctx.tapleaf_hash.to_byte_array()),
        11 => byte_atom(&ctx.internal_key.serialize()),
        other => return Err(EvalError::UnknownTxField(other)),
    };
    Ok(value)
}

/// `(bip342_txmsg [SH])` — the BIP341 script-path sighash for the current input.
///
/// v0,1 supports only `SIGHASH_DEFAULT` (an absent, nil, or `0x00` flag).
pub(crate) fn bip342_txmsg(args: &[Value], ctx: Option<&TxContext>) -> Result<Value, EvalError> {
    if args.len() > 1 {
        return Err(EvalError::Arity {
            op: "bip342_txmsg",
            expected: 1,
            found: args.len(),
        });
    }
    if let Some(flag) = args.first() {
        let bytes = flag
            .as_atom_bytes()
            .ok_or(EvalError::TypeError("sighash flag must be an atom"))?;
        if !bytes.is_empty() && bytes != [0x00] {
            return Err(EvalError::NotImplemented("non-default sighash"));
        }
    }

    let ctx = ctx.ok_or(EvalError::NoTxContext)?;
    let mut cache = SighashCache::new(ctx.tx);
    let sighash = cache
        .taproot_script_spend_signature_hash(
            ctx.input_index as usize,
            &Prevouts::All(ctx.prevouts),
            ctx.tapleaf_hash,
            TapSighashType::Default,
        )
        .map_err(|e| EvalError::SighashError(e.to_string()))?;
    Ok(byte_atom(&sighash.to_byte_array()))
}

/// A byte slice as a value atom (empty becomes `nil`).
fn byte_atom(bytes: &[u8]) -> Value {
    if bytes.is_empty() {
        Value::Nil
    } else {
        Value::atom(bytes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::absolute::LockTime;
    use bitcoin::transaction::Version;
    use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, Witness};

    fn sample() -> (Transaction, Vec<TxOut>) {
        let tx = Transaction {
            version: Version::TWO,
            lock_time: LockTime::from_consensus(500_000),
            input: vec![TxIn {
                previous_output: OutPoint::null(),
                script_sig: ScriptBuf::new(),
                sequence: Sequence(7),
                witness: Witness::new(),
            }],
            output: vec![TxOut {
                value: Amount::from_sat(40_000),
                script_pubkey: ScriptBuf::new(),
            }],
        };
        let prevouts = vec![TxOut {
            value: Amount::from_sat(100_000),
            script_pubkey: ScriptBuf::new(),
        }];
        (tx, prevouts)
    }

    fn ctx<'a>(tx: &'a Transaction, prevouts: &'a [TxOut]) -> TxContext<'a> {
        TxContext {
            tx,
            input_index: 0,
            prevouts,
            tapleaf_hash: TapLeafHash::from_byte_array([0u8; 32]),
            internal_key: XOnlyPublicKey::from_slice(&[2u8; 32]).unwrap_or_else(|_| {
                // 0x02..02 is a valid x-only key; fall back is unreachable.
                unreachable!("fixed valid x-only key")
            }),
            merkle_branch: Vec::new(),
        }
    }

    #[test]
    fn reads_tx_fields() {
        let (tx, prevouts) = sample();
        let c = ctx(&tx, &prevouts);
        assert_eq!(
            super::tx(&[Value::from_u64(0)], Some(&c)).unwrap(),
            Value::from_u64(2)
        );
        assert_eq!(
            super::tx(&[Value::from_u64(1)], Some(&c)).unwrap(),
            Value::from_u64(500_000)
        );
        assert_eq!(
            super::tx(&[Value::from_u64(2)], Some(&c)).unwrap(),
            Value::from_u64(1)
        );
        assert_eq!(
            super::tx(&[Value::from_u64(5)], Some(&c)).unwrap(),
            Value::from_u64(7)
        );
        assert_eq!(
            super::tx(&[Value::from_u64(8)], Some(&c)).unwrap(),
            Value::from_u64(100_000)
        );
    }

    #[test]
    fn unknown_field_errors() {
        let (tx, prevouts) = sample();
        let c = ctx(&tx, &prevouts);
        assert_eq!(
            super::tx(&[Value::from_u64(99)], Some(&c)),
            Err(EvalError::UnknownTxField(99))
        );
    }

    #[test]
    fn no_context_errors() {
        assert_eq!(
            super::tx(&[Value::from_u64(0)], None),
            Err(EvalError::NoTxContext)
        );
        assert_eq!(bip342_txmsg(&[], None), Err(EvalError::NoTxContext));
    }

    #[test]
    fn bip342_returns_32_bytes() {
        let (tx, prevouts) = sample();
        let c = ctx(&tx, &prevouts);
        let msg = bip342_txmsg(&[], Some(&c)).unwrap();
        assert_eq!(msg.as_atom_bytes().map(<[u8]>::len), Some(32));
    }
}
