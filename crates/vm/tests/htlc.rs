//! A realistic end-to-end contract: an HTLC (Hash Time-Locked Contract), the
//! building block of Lightning. Bob claims by revealing the preimage and
//! signing; Alice refunds after the deadline by signing. Contract parameters
//! (hash, pubkeys, deadline) are literals committed in the tapleaf; the witness
//! supplies `(claim preimage bob_sig alice_sig)`.

use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash as _;
use bitcoin::secp256k1::{Keypair, Message, Secp256k1, SecretKey};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::LeafVersion;
use bitcoin::transaction::Version;
use bitcoin::{
    Amount, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut, Witness,
};
use sha2::{Digest, Sha256};

use btclisp_compiler::compile;
use btclisp_core::CoreExpr;
use btclisp_vm::{eval, EvalCtx, TxContext, Value};

const BOB_SK: [u8; 32] = [0x11; 32];
const ALICE_SK: [u8; 32] = [0x22; 32];
const PREIMAGE: &[u8] = b"btclisp";
const DEADLINE: u32 = 750_000;

fn secp() -> Secp256k1<bitcoin::secp256k1::All> {
    Secp256k1::new()
}

fn keypair(sk: &[u8; 32]) -> Keypair {
    Keypair::from_secret_key(&secp(), &SecretKey::from_slice(sk).unwrap())
}

fn hexstr(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut a, b| {
        let _ = write!(a, "{b:02x}");
        a
    })
}

/// Build the HTLC source with concrete committed parameters.
fn htlc_source() -> String {
    let bob_pk = keypair(&BOB_SK).x_only_public_key().0.serialize();
    let alice_pk = keypair(&ALICE_SK).x_only_public_key().0.serialize();
    let hash = Sha256::digest(PREIMAGE);
    format!(
        "(if claim\n    \
            (all (= (sha256 preimage) 0x{hash})\n         \
                 (bip340_verify 0x{bob} (bip342_txmsg) bob_sig))\n    \
            (all (< {DEADLINE} (tx 1))\n         \
                 (bip340_verify 0x{alice} (bip342_txmsg) alice_sig)))\n",
        hash = hexstr(&hash),
        bob = hexstr(&bob_pk),
        alice = hexstr(&alice_pk),
    )
}

/// A four-element environment list `(claim preimage bob_sig alice_sig)`.
fn env(claim: Value, preimage: Value, bob_sig: Value, alice_sig: Value) -> Value {
    Value::cons(
        claim,
        Value::cons(
            preimage,
            Value::cons(bob_sig, Value::cons(alice_sig, Value::Nil)),
        ),
    )
}

/// Build a 1-in/1-out spend with the given lock time and compute the BIP341
/// script-path sighash for the HTLC leaf.
fn spend_and_sighash(
    leaf: &ScriptBuf,
    lock_time: u32,
) -> (Transaction, [TxOut; 1], TapLeafHash, [u8; 32]) {
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_consensus(lock_time),
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            // CLTV requires a non-final sequence; harmless for the claim branch.
            sequence: Sequence::ENABLE_LOCKTIME_NO_RBF,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(45_000),
            script_pubkey: ScriptBuf::new(),
        }],
    };
    let prevouts = [TxOut {
        value: Amount::from_sat(50_000),
        script_pubkey: ScriptBuf::new(),
    }];
    let tapleaf = TapLeafHash::from_script(leaf.as_script(), LeafVersion::TapScript);
    let sighash = SighashCache::new(&tx)
        .taproot_script_spend_signature_hash(
            0,
            &Prevouts::All(&prevouts),
            tapleaf,
            TapSighashType::Default,
        )
        .unwrap()
        .to_byte_array();
    (tx, prevouts, tapleaf, sighash)
}

fn sign(sk: &[u8; 32], sighash: &[u8; 32]) -> Vec<u8> {
    secp()
        .sign_schnorr_no_aux_rand(&Message::from_digest(*sighash), &keypair(sk))
        .serialize()
        .to_vec()
}

fn run(
    program: &CoreExpr,
    tx: &Transaction,
    prevouts: &[TxOut],
    tapleaf: TapLeafHash,
    env: &Value,
) -> Value {
    let txc = TxContext {
        tx,
        input_index: 0,
        prevouts,
        tapleaf_hash: tapleaf,
        internal_key: keypair(&BOB_SK).x_only_public_key().0,
        merkle_branch: Vec::new(),
    };
    let mut ctx = EvalCtx::with_tx_context(&txc);
    eval(program, env, &mut ctx).expect("eval")
}

#[test]
fn example_file_is_in_sync() {
    // The committed example must compile to the same program as the generated
    // source (comments/whitespace aside), so the literals never drift.
    let from_file = compile(include_str!("../../../examples/08_htlc.btl")).unwrap();
    let generated = compile(&htlc_source()).unwrap();
    assert_eq!(from_file.core, generated.core);
}

#[test]
fn bob_claims_with_correct_preimage() {
    let program = compile(&htlc_source()).unwrap();
    let leaf = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));
    let (tx, prevouts, tapleaf, sighash) = spend_and_sighash(&leaf, 0);
    let env = env(
        Value::atom(vec![1]),
        Value::atom(PREIMAGE.to_vec()),
        Value::atom(sign(&BOB_SK, &sighash)),
        Value::Nil,
    );
    assert_eq!(
        run(&program.core, &tx, &prevouts, tapleaf, &env),
        Value::atom(vec![1])
    );
}

#[test]
fn alice_refunds_after_deadline() {
    let program = compile(&htlc_source()).unwrap();
    let leaf = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));
    let (tx, prevouts, tapleaf, sighash) = spend_and_sighash(&leaf, DEADLINE + 1);
    let env = env(
        Value::Nil,
        Value::Nil,
        Value::Nil,
        Value::atom(sign(&ALICE_SK, &sighash)),
    );
    assert_eq!(
        run(&program.core, &tx, &prevouts, tapleaf, &env),
        Value::atom(vec![1])
    );
}

#[test]
fn claim_fails_with_wrong_preimage() {
    let program = compile(&htlc_source()).unwrap();
    let leaf = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));
    let (tx, prevouts, tapleaf, sighash) = spend_and_sighash(&leaf, 0);
    let env = env(
        Value::atom(vec![1]),
        Value::atom(b"wrong".to_vec()),
        Value::atom(sign(&BOB_SK, &sighash)),
        Value::Nil,
    );
    // The hash check fails, so `all` is nil (false) — the claim is rejected.
    assert_eq!(
        run(&program.core, &tx, &prevouts, tapleaf, &env),
        Value::Nil
    );
}

#[test]
fn refund_fails_before_deadline() {
    let program = compile(&htlc_source()).unwrap();
    let leaf = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));
    let (tx, prevouts, tapleaf, sighash) = spend_and_sighash(&leaf, DEADLINE - 1);
    let env = env(
        Value::Nil,
        Value::Nil,
        Value::Nil,
        Value::atom(sign(&ALICE_SK, &sighash)),
    );
    // nLockTime has not passed the deadline, so the timelock check is false.
    assert_eq!(
        run(&program.core, &tx, &prevouts, tapleaf, &env),
        Value::Nil
    );
}
