//! Regtest integration harness (spec §10), gated behind `--features regtest`.
//!
//! Drives a live [Nigiri](https://nigiri.vulpem.com) regtest node (Docker
//! bitcoind). Run with:
//!
//! ```text
//! nigiri start
//! cargo test -p btclisp-cli --features regtest -- --nocapture
//! ```
//!
//! What it proves:
//! - a real P2TR output committing to a `btclispc`-compiled program as a
//!   tapleaf can be funded and spent (via the key path) on real bitcoind, i.e.
//!   `sendrawtransaction` succeeds;
//! - our interpreter validates the *script-path* witness off-chain — what a
//!   btclisp-aware validator would enforce. (bitcoind cannot execute btclisp,
//!   which is a proposed opcode set, so the on-chain settlement uses the key
//!   path while the script semantics are checked by our VM.)
#![cfg(feature = "regtest")]

use std::process::Command;
use std::str::FromStr;

use bitcoin::absolute::LockTime;
use bitcoin::hashes::Hash as _;
use bitcoin::key::TapTweak as _;
use bitcoin::secp256k1::{Keypair, Message, Secp256k1};
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::{LeafVersion, TaprootBuilder};
use bitcoin::transaction::Version;
use bitcoin::{
    Address, Amount, Network, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut,
    Witness, XOnlyPublicKey,
};

use btclisp_compiler::compile;

/// Run `nigiri <args>` and return trimmed stdout, panicking on failure.
///
/// Nigiri colorizes JSON output even when piped, so ANSI escapes are stripped.
fn nigiri(args: &[&str]) -> String {
    let output = Command::new("nigiri")
        .args(args)
        .output()
        .expect("`nigiri` must be on PATH");
    assert!(
        output.status.success(),
        "nigiri {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    strip_ansi(&String::from_utf8(output.stdout).unwrap())
        .trim()
        .to_string()
}

/// Remove ANSI CSI escape sequences (`ESC [ ... m`).
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for n in chars.by_ref() {
                if n == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn rpc(args: &[&str]) -> String {
    let mut full = vec!["rpc"];
    full.extend_from_slice(args);
    nigiri(&full)
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

fn keypair(secp: &Secp256k1<bitcoin::secp256k1::All>, seed: u8) -> Keypair {
    Keypair::from_seckey_slice(secp, &[seed; 32]).expect("valid secret key")
}

#[test]
fn taproot_btclisp_commitment_spends_on_regtest() {
    let secp = Secp256k1::new();

    // Compile a btclisp program; it becomes the committed tapleaf script.
    let program = compile("(bip340_verify pk (bip342_txmsg) sig)").expect("compile");
    let leaf_script = ScriptBuf::from_bytes(btclisp_codec::encode(&program.core));

    // Build a Taproot output committing to the program under an internal key.
    let internal = keypair(&secp, 0x11);
    let internal_xonly = internal.x_only_public_key().0;
    let spend_info = TaprootBuilder::new()
        .add_leaf(0, leaf_script.clone())
        .expect("add leaf")
        .finalize(&secp, internal_xonly)
        .expect("finalize taproot");
    let address = Address::p2tr_tweaked(spend_info.output_key(), Network::Regtest);
    let spk = address.script_pubkey();
    let spk_hex = hex_encode(spk.as_bytes());

    // Fund the address on regtest (Nigiri's faucet also mines a block).
    let faucet = nigiri(&["faucet", &address.to_string(), "0.001"]);
    let funding_txid = faucet
        .split_whitespace()
        .last()
        .expect("faucet txid")
        .to_string();

    // Locate the funding output paying our address.
    let raw = rpc(&["getrawtransaction", &funding_txid, "true"]);
    let tx_json: serde_json::Value = serde_json::from_str(&raw).expect("tx json");
    let (vout, sats) = tx_json["vout"]
        .as_array()
        .expect("vout array")
        .iter()
        .find_map(|out| {
            (out["scriptPubKey"]["hex"].as_str()? == spk_hex).then(|| {
                let n = out["n"].as_u64().unwrap();
                let sats = (out["value"].as_f64().unwrap() * 1e8).round() as u64;
                (u32::try_from(n).unwrap(), sats)
            })
        })
        .expect("funding output for our address");

    let prevout = TxOut {
        value: Amount::from_sat(sats),
        script_pubkey: spk.clone(),
    };

    // Spend it back to a fresh node address via the key path.
    let dest = rpc(&["getnewaddress", "", "bech32m"]);
    let dest_spk = Address::from_str(&dest)
        .unwrap()
        .assume_checked()
        .script_pubkey();

    let mut spend = Transaction {
        version: Version::TWO,
        lock_time: LockTime::ZERO,
        input: vec![TxIn {
            previous_output: OutPoint {
                txid: funding_txid.parse().unwrap(),
                vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value: Amount::from_sat(sats - 1_000),
            script_pubkey: dest_spk,
        }],
    };

    let sighash = SighashCache::new(&spend)
        .taproot_key_spend_signature_hash(
            0,
            &Prevouts::All(std::slice::from_ref(&prevout)),
            TapSighashType::Default,
        )
        .expect("key-spend sighash");
    let tweaked = internal.tap_tweak(&secp, spend_info.merkle_root());
    let signature = secp.sign_schnorr_no_aux_rand(
        &Message::from_digest(sighash.to_byte_array()),
        &tweaked.to_keypair(),
    );
    let mut witness = Witness::new();
    witness.push(signature.serialize());
    spend.input[0].witness = witness;

    // Broadcasting must succeed: the btclisp-committed output is real and spendable.
    let raw_hex = bitcoin::consensus::encode::serialize_hex(&spend);
    let spent_txid = rpc(&["sendrawtransaction", &raw_hex]);
    assert_eq!(spent_txid.len(), 64, "expected a txid, got: {spent_txid}");

    // Off-chain: validate the script-path witness with our VM, proving what a
    // btclisp-aware validator would enforce for the same output.
    validate_script_path_offchain(&secp, &spend, &prevout, &leaf_script);

    println!("regtest key-path spend broadcast: {spent_txid}");
}

/// Sign the BIP342 script-path sighash and run the program through the VM.
fn validate_script_path_offchain(
    secp: &Secp256k1<bitcoin::secp256k1::All>,
    spend: &Transaction,
    prevout: &TxOut,
    leaf_script: &ScriptBuf,
) {
    use btclisp_vm::{eval, EvalCtx, TxContext, Value};

    let signer = keypair(secp, 0x22);
    let pk = signer.x_only_public_key().0;
    let tapleaf_hash = TapLeafHash::from_script(leaf_script.as_script(), LeafVersion::TapScript);

    let prevouts = [prevout.clone()];
    let internal = XOnlyPublicKey::from_slice(&[2u8; 32]).unwrap();
    let txc = TxContext {
        tx: spend,
        input_index: 0,
        prevouts: &prevouts,
        tapleaf_hash,
        internal_key: internal,
        merkle_branch: Vec::new(),
    };

    let program = compile("(bip340_verify pk (bip342_txmsg) sig)").unwrap();
    let msg = eval(
        &compile("(bip342_txmsg)").unwrap().core,
        &Value::Nil,
        &mut EvalCtx::with_tx_context(&txc),
    )
    .unwrap();
    let message = Message::from_digest_slice(msg.as_atom_bytes().unwrap()).unwrap();
    let sig = secp.sign_schnorr_no_aux_rand(&message, &signer);

    let env = Value::cons(
        Value::atom(pk.serialize().to_vec()),
        Value::cons(Value::atom(sig.serialize().to_vec()), Value::Nil),
    );
    let result = eval(&program.core, &env, &mut EvalCtx::with_tx_context(&txc)).unwrap();
    assert_eq!(
        result,
        Value::atom(vec![1]),
        "off-chain script-path validation failed"
    );
}
