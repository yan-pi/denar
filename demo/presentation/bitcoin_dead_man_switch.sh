#!/usr/bin/env bash
set -euo pipefail

# Bitcoin-focused dead man's switch demo.
#
# This script keeps the real btclisp program:
#
#   (if (< (tx 1) deadline)
#       (bip340_verify owner_pk (bip342_txmsg) owner_sig)
#       (bip340_verify heir_pk (bip342_txmsg) heir_sig))
#
# and supplies the missing Bitcoin context for `btclispc run`:
#
# - a raw spending transaction (`--tx`);
# - prevouts JSON (`--prevouts`);
# - a serialized btclisp environment (`--env`) containing:
#   deadline owner_pk owner_sig heir_pk heir_sig.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
DEFAULT_SOURCE="$ROOT_DIR/examples/08_dead_man_switch.btl"
if [[ ! -f "$DEFAULT_SOURCE" ]]; then
  DEFAULT_SOURCE="$SCRIPT_DIR/08_dead_man_switch.btl"
fi
SOURCE="${SOURCE:-$DEFAULT_SOURCE}"
PROGRAM_BIN="${PROGRAM_BIN:-/tmp/08_dead_man_switch.bin}"
WORK_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$WORK_DIR"
}
trap cleanup EXIT

FIXTURE_DIR="$WORK_DIR/fixture-gen"
PREVOUTS_JSON="$WORK_DIR/prevouts.json"

mkdir -p "$FIXTURE_DIR/src"

cat > "$FIXTURE_DIR/Cargo.toml" <<EOF
[package]
name = "btclisp-deadman-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
btclisp-codec = { path = "$ROOT_DIR/crates/codec" }
btclisp-core = { path = "$ROOT_DIR/crates/core" }
bitcoin = "0.32"
secp256k1 = "0.29"
EOF

cat > "$FIXTURE_DIR/src/main.rs" <<'RS'
use std::{env, fs};

use bitcoin::absolute::LockTime;
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::Hash as _;
use bitcoin::sighash::{Prevouts, SighashCache, TapSighashType};
use bitcoin::taproot::LeafVersion;
use bitcoin::transaction::Version;
use bitcoin::{Amount, OutPoint, ScriptBuf, Sequence, TapLeafHash, Transaction, TxIn, TxOut, Witness};
use btclisp_core::CoreExpr;
use secp256k1::{Keypair, Message, Secp256k1};

const DEADLINE: u64 = 600_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        eprintln!("usage: fixture-gen <program-bin> <before|after> <prevouts-json>");
        std::process::exit(2);
    }

    let program_bytes = fs::read(&args[1])?;
    let scenario = args[2].as_str();
    let prevouts_json = &args[3];
    let lock_time = match scenario {
        "before" => 500_000,
        "after" => 700_000,
        other => return Err(format!("unknown scenario `{other}`").into()),
    };

    let secp = Secp256k1::new();
    let owner = Keypair::from_seckey_slice(&secp, &[0x11; 32])?;
    let heir = Keypair::from_seckey_slice(&secp, &[0x22; 32])?;

    let tx = Transaction {
        version: Version::TWO,
        lock_time: LockTime::from_consensus(lock_time),
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
    fs::write(prevouts_json, r#"[{"value":100000,"script_pubkey":""}]"#)?;

    let tapleaf_hash = TapLeafHash::from_script(
        ScriptBuf::from_bytes(program_bytes).as_script(),
        LeafVersion::TapScript,
    );
    let sighash = SighashCache::new(&tx).taproot_script_spend_signature_hash(
        0,
        &Prevouts::All(&prevouts),
        tapleaf_hash,
        TapSighashType::Default,
    )?;
    let message = Message::from_digest(sighash.to_byte_array());

    let owner_pk = owner.x_only_public_key().0.serialize().to_vec();
    let heir_pk = heir.x_only_public_key().0.serialize().to_vec();
    let owner_sig = secp
        .sign_schnorr_no_aux_rand(&message, &owner)
        .serialize()
        .to_vec();
    let heir_sig = secp
        .sign_schnorr_no_aux_rand(&message, &heir)
        .serialize()
        .to_vec();

    let env = match scenario {
        "before" => proper_list(vec![
            int_value(DEADLINE),
            CoreExpr::atom(owner_pk),
            CoreExpr::atom(owner_sig),
            CoreExpr::atom(heir_pk),
            CoreExpr::Nil,
        ]),
        "after" => proper_list(vec![
            int_value(DEADLINE),
            CoreExpr::atom(owner_pk),
            CoreExpr::Nil,
            CoreExpr::atom(heir_pk),
            CoreExpr::atom(heir_sig),
        ]),
        _ => unreachable!(),
    };

    println!("SCENARIO={scenario}");
    println!("LOCKTIME={lock_time}");
    println!("DEADLINE={DEADLINE}");
    println!("TX_HEX={}", serialize_hex(&tx));
    println!("ENV_HEX={}", hex(&btclisp_codec::encode(&env)));
    Ok(())
}

fn proper_list(items: Vec<CoreExpr>) -> CoreExpr {
    items
        .into_iter()
        .rev()
        .fold(CoreExpr::Nil, |tail, item| CoreExpr::cons(item, tail))
}

fn int_value(n: u64) -> CoreExpr {
    let bytes = le_minimal(n);
    if bytes.is_empty() {
        CoreExpr::Nil
    } else {
        CoreExpr::atom(bytes)
    }
}

fn le_minimal(n: u64) -> Vec<u8> {
    let mut bytes = n.to_le_bytes().to_vec();
    while bytes.last() == Some(&0) {
        bytes.pop();
    }
    bytes
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}
RS

fixture_value() {
  local output="$1"
  local key="$2"
  local line

  while IFS= read -r line; do
    case "$line" in
      "$key="*)
        printf '%s\n' "${line#*=}"
        return 0
        ;;
    esac
  done <<< "$output"

  printf 'missing %s in fixture output\n' "$key" >&2
  return 1
}

run_scenario() {
  local scenario="$1"
  local fixture_output
  local tx_hex
  local env_hex
  local locktime
  local deadline

  fixture_output="$(cargo run --quiet --manifest-path "$FIXTURE_DIR/Cargo.toml" -- "$PROGRAM_BIN" "$scenario" "$PREVOUTS_JSON")"
  tx_hex="$(fixture_value "$fixture_output" TX_HEX)"
  env_hex="$(fixture_value "$fixture_output" ENV_HEX)"
  locktime="$(fixture_value "$fixture_output" LOCKTIME)"
  deadline="$(fixture_value "$fixture_output" DEADLINE)"

  printf '\n== %s deadline ==\n' "$scenario"
  printf 'locktime=%s deadline=%s\n' "$locktime" "$deadline"
  cargo run --quiet -p btclisp-cli -- run "$PROGRAM_BIN" \
    --env "$env_hex" \
    --tx "$tx_hex" \
    --prevouts "$PREVOUTS_JSON" \
    --input-index 0
}

printf 'source: %s\n' "$SOURCE"
printf 'program: %s\n' "$PROGRAM_BIN"

cargo run --quiet -p btclisp-cli -- check --core "$SOURCE"
cargo run --quiet -p btclisp-cli -- compile "$SOURCE" -o "$PROGRAM_BIN"
cargo run --quiet -p btclisp-cli -- disasm "$PROGRAM_BIN"

run_scenario before
run_scenario after
