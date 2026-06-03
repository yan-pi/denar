//! End-to-end evaluation tests: compile btclisp source, then run it.

use btclisp_compiler::compile;
use btclisp_vm::{eval, EvalCtx, EvalError, TxContext, Value};

/// Compile `src` and evaluate it against `env`.
fn run(src: &str, env: &Value) -> Result<Value, EvalError> {
    let program = compile(src).expect("compile");
    let mut ctx = EvalCtx::default();
    eval(&program.core, env, &mut ctx)
}

/// Compile and evaluate against an empty environment.
fn run_nil(src: &str) -> Result<Value, EvalError> {
    run(src, &Value::Nil)
}

#[test]
fn arithmetic() {
    assert_eq!(run_nil("(+ 2 3)").unwrap(), Value::from_u64(5));
    assert_eq!(run_nil("(* (+ 1 2) 4)").unwrap(), Value::from_u64(12));
    assert_eq!(run_nil("(- 10 3 2)").unwrap(), Value::from_u64(5));
    assert_eq!(run_nil("(% 17 5)").unwrap(), Value::from_u64(2));
    assert_eq!(run_nil("(/ 17 5)").unwrap(), Value::from_u64(3));
}

#[test]
fn comparisons_and_booleans() {
    let t = Value::atom(vec![1]);
    assert_eq!(run_nil("(< 1 2)").unwrap(), t);
    assert_eq!(run_nil("(< 2 2)").unwrap(), Value::Nil);
    assert_eq!(run_nil("(= 5 5)").unwrap(), t);
    assert_eq!(run_nil("(not nil)").unwrap(), t);
    assert_eq!(run_nil("(all 1 2 3)").unwrap(), t);
    assert_eq!(run_nil("(any nil nil)").unwrap(), Value::Nil);
}

#[test]
fn if_and_cond() {
    assert_eq!(run_nil("(if (< 1 2) 10 20)").unwrap(), Value::from_u64(10));
    assert_eq!(run_nil("(if (< 2 1) 10 20)").unwrap(), Value::from_u64(20));
    assert_eq!(
        run_nil("(cond ((< 5 1) 100) ((= 5 5) 200))").unwrap(),
        Value::from_u64(200)
    );
}

#[test]
fn let_binding() {
    assert_eq!(
        run_nil("(let ((x 4) (y 5)) (+ x y))").unwrap(),
        Value::from_u64(9)
    );
}

#[test]
fn function_inlining() {
    assert_eq!(
        run_nil("(defun sq (n) (* n n))\n(+ (sq 3) (sq 4))").unwrap(),
        Value::from_u64(25)
    );
}

#[test]
fn list_operations() {
    // (h (t (c 1 (c 2 nil)))) -> 2
    assert_eq!(
        run_nil("(h (t (c 1 (c 2 nil))))").unwrap(),
        Value::from_u64(2)
    );
    assert_eq!(run_nil("(l (c 1 nil))").unwrap(), Value::atom(vec![1]));
    assert_eq!(run_nil("(l 5)").unwrap(), Value::Nil);
}

#[test]
fn byte_operations() {
    assert_eq!(
        run_nil("(cat 0xaa 0xbbcc)").unwrap(),
        Value::atom(vec![0xaa, 0xbb, 0xcc])
    );
    assert_eq!(run_nil("(strlen 0xaabb 0xcc)").unwrap(), Value::from_u64(3));
    assert_eq!(
        run_nil("(substr 0x00112233 1 3)").unwrap(),
        Value::atom(vec![0x11, 0x22])
    );
}

#[test]
fn rd_wr_round_trip() {
    // (rd (wr '(+ 1 2))) reconstructs the quoted datum list (0x17 1 2).
    let result = run_nil("(rd (wr '(+ 1 2)))").unwrap();
    let expected = Value::cons(
        Value::atom(vec![0x17]),
        Value::cons(
            Value::atom(vec![1]),
            Value::cons(Value::atom(vec![2]), Value::Nil),
        ),
    );
    assert_eq!(result, expected);
}

#[test]
fn top_level_env_parameters() {
    // (+ a b) with env = (3 . (4 . nil)) -> 7
    let env = Value::cons(
        Value::from_u64(3),
        Value::cons(Value::from_u64(4), Value::Nil),
    );
    assert_eq!(run("(+ a b)", &env).unwrap(), Value::from_u64(7));
}

#[test]
fn hashlock_predicate_shape() {
    // (= preimage target) with env = (k . (k . nil)) -> true
    let env = Value::cons(
        Value::atom(vec![0xab]),
        Value::cons(Value::atom(vec![0xab]), Value::Nil),
    );
    assert_eq!(
        run("(= preimage target)", &env).unwrap(),
        Value::atom(vec![1])
    );
}

#[test]
fn explicit_exception() {
    assert_eq!(run_nil("(x)"), Err(EvalError::Exception));
}

fn hx(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn hashlock_predicate() {
    // (= (sha256 preimage) target); env = (preimage . (target . nil)).
    let preimage = Value::atom(b"abc".to_vec());
    let target = Value::atom(hx(
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    ));
    let env = Value::cons(preimage, Value::cons(target, Value::Nil));
    assert_eq!(
        run("(= (sha256 preimage) target)", &env).unwrap(),
        Value::atom(vec![1])
    );
}

#[test]
fn p2pk_taproot_spend_verifies() {
    use bitcoin::absolute::LockTime;
    use bitcoin::taproot::LeafVersion;
    use bitcoin::transaction::Version;
    use bitcoin::{
        Amount, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut, Witness,
    };
    use secp256k1::{Keypair, Message, Secp256k1};

    // A signing key and its x-only public key.
    let secp = Secp256k1::signing_only();
    let keypair = Keypair::from_seckey_slice(&secp, &[0x42u8; 32]).unwrap();
    let (xonly, _parity) = keypair.x_only_public_key();

    // A 1-in/1-out spending transaction and its prevout.
    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
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

    // The tapleaf commits to the compiled program bytes.
    let program = compile("(bip340_verify pk (bip342_txmsg) sig)").unwrap();
    let script = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));
    let tapleaf_hash = TapLeafHash::from_script(script.as_script(), LeafVersion::TapScript);

    let txc = TxContext {
        tx: &tx,
        input_index: 0,
        prevouts: &prevouts,
        tapleaf_hash,
        internal_key: xonly,
        merkle_branch: Vec::new(),
    };

    // Obtain the sighash by evaluating (bip342_txmsg), then sign it.
    let msg_program = compile("(bip342_txmsg)").unwrap();
    let mut ctx = EvalCtx::with_tx_context(&txc);
    let msg_value = eval(&msg_program.core, &Value::Nil, &mut ctx).unwrap();
    let msg_bytes = msg_value.as_atom_bytes().unwrap();
    let message = Message::from_digest_slice(msg_bytes).unwrap();
    let signature = secp.sign_schnorr_no_aux_rand(&message, &keypair);

    // The witness env supplies the public key and the signature.
    let env = Value::cons(
        Value::atom(xonly.serialize().to_vec()),
        Value::cons(Value::atom(signature.serialize().to_vec()), Value::Nil),
    );
    let mut ctx = EvalCtx::with_tx_context(&txc);
    let result = eval(&program.core, &env, &mut ctx).unwrap();
    assert_eq!(result, Value::atom(vec![1]));
}

#[test]
fn multisig_2of3_two_valid_signatures() {
    use bitcoin::absolute::LockTime;
    use bitcoin::taproot::LeafVersion;
    use bitcoin::transaction::Version;
    use bitcoin::{
        Amount, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut, Witness,
    };
    use secp256k1::{Keypair, Message, Secp256k1};

    let secp = Secp256k1::signing_only();
    let keys: Vec<Keypair> = [[0x11u8; 32], [0x22u8; 32], [0x33u8; 32]]
        .iter()
        .map(|sk| Keypair::from_seckey_slice(&secp, sk).unwrap())
        .collect();

    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(10_000),
            script_pubkey: ScriptBuf::new(),
        }],
    };
    let prevouts = vec![TxOut {
        value: Amount::from_sat(20_000),
        script_pubkey: ScriptBuf::new(),
    }];

    let program = compile(include_str!("../../../examples/06_multisig_2of3.btl")).unwrap();
    let script = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));
    let tapleaf_hash = TapLeafHash::from_script(script.as_script(), LeafVersion::TapScript);
    let txc = TxContext {
        tx: &tx,
        input_index: 0,
        prevouts: &prevouts,
        tapleaf_hash,
        internal_key: keys[0].x_only_public_key().0,
        merkle_branch: Vec::new(),
    };

    // Compute the shared sighash via (bip342_txmsg).
    let mut ctx = EvalCtx::with_tx_context(&txc);
    let msg_value = eval(
        &compile("(bip342_txmsg)").unwrap().core,
        &Value::Nil,
        &mut ctx,
    )
    .unwrap();
    let message = Message::from_digest_slice(msg_value.as_atom_bytes().unwrap()).unwrap();

    // Signers 1 and 2 sign; signer 3 provides an empty signature.
    let sign = |kp: &Keypair| {
        secp.sign_schnorr_no_aux_rand(&message, kp)
            .serialize()
            .to_vec()
    };
    let pk = |kp: &Keypair| kp.x_only_public_key().0.serialize().to_vec();

    // env order (first appearance): pk1 sig1 pk2 sig2 pk3 sig3.
    let env_items = vec![
        Value::atom(pk(&keys[0])),
        Value::atom(sign(&keys[0])),
        Value::atom(pk(&keys[1])),
        Value::atom(sign(&keys[1])),
        Value::atom(pk(&keys[2])),
        Value::Nil,
    ];
    let mut env = Value::Nil;
    for item in env_items.into_iter().rev() {
        env = Value::cons(item, env);
    }

    let mut ctx = EvalCtx::with_tx_context(&txc);
    assert_eq!(
        eval(&program.core, &env, &mut ctx).unwrap(),
        Value::atom(vec![1])
    );
}

#[test]
fn csv_timelock_reads_locktime() {
    use bitcoin::absolute::LockTime;
    use bitcoin::hashes::Hash as _;
    use bitcoin::transaction::Version;
    use bitcoin::{
        Amount, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut, Witness,
    };
    use secp256k1::XOnlyPublicKey;

    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_consensus(600_000),
        input: vec![TxIn {
            previous_output: OutPoint::null(),
            script_sig: ScriptBuf::new(),
            sequence: Sequence::MAX,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(1),
            script_pubkey: ScriptBuf::new(),
        }],
    };
    let prevouts = vec![TxOut {
        value: Amount::from_sat(2),
        script_pubkey: ScriptBuf::new(),
    }];
    let txc = TxContext {
        tx: &tx,
        input_index: 0,
        prevouts: &prevouts,
        tapleaf_hash: TapLeafHash::from_byte_array([0u8; 32]),
        internal_key: XOnlyPublicKey::from_slice(&[2u8; 32]).unwrap(),
        merkle_branch: Vec::new(),
    };

    // (< deadline (tx 1)) with deadline = 500_000 in env -> 500_000 < 600_000 -> true.
    let program = compile("(< deadline (tx 1))").unwrap();
    let env = Value::cons(Value::from_u64(500_000), Value::Nil);
    let mut ctx = EvalCtx::with_tx_context(&txc);
    assert_eq!(
        eval(&program.core, &env, &mut ctx).unwrap(),
        Value::atom(vec![1])
    );
}

#[test]
fn offline_p2pk_with_bip340_vector() {
    // (bip340_verify pk msg sig) with BIP340 vector 0 as the env.
    let pk = Value::atom(hx(
        "F9308A019258C31049344F85F89D5229B531C845836F99B08601F113BCE036F9",
    ));
    let msg = Value::atom(hx(
        "0000000000000000000000000000000000000000000000000000000000000000",
    ));
    let sig = Value::atom(hx(
        "E907831F80848D1069A5371B402410364BDF1C5F8307B0084C55F1CE2DCA821525F66A4A85EA8B71E482A74F382D2CE5EBEEE8FDB2172F477DF4900D310536C0",
    ));
    let env = Value::cons(pk, Value::cons(msg, Value::cons(sig, Value::Nil)));
    assert_eq!(
        run("(bip340_verify pk msg sig)", &env).unwrap(),
        Value::atom(vec![1])
    );
}
