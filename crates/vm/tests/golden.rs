//! Golden corpus harness (spec §10).
//!
//! For every `examples/NN_name.btl`:
//! - compile it and assert the bytes match `NN_name.bin.hex`;
//! - if `NN_name.expected` exists, evaluate it and assert the result.
//!
//! `.expected` is a set of `key value` lines:
//! - `env <btclisp-expr>` — evaluated against nil to build the environment;
//! - `tx <hex>` — raw transaction enabling `tx`/`bip342_txmsg`;
//! - `result <s-expr>` or `result FAIL`.
//!
//! Examples needing real signatures (03, 06) ship only `.bin.hex`; their
//! evaluation lives in `tests/eval.rs`.

use std::fs;
use std::path::PathBuf;

use btclisp_compiler::compile;
use btclisp_vm::{eval, EvalCtx, TxContext, Value};

const NUMS_KEY_HEX: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

fn hex_decode(s: &str) -> Vec<u8> {
    let s = s.trim();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex"))
        .collect()
}

#[test]
fn golden_corpus() {
    let dir = examples_dir();
    let mut checked = 0;

    let mut entries: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("read examples dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("btl"))
        .collect();
    entries.sort();

    for path in entries {
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("stem");
        let src = fs::read_to_string(&path).expect("read source");
        let program = compile(&src).unwrap_or_else(|e| panic!("compile {stem}: {e}"));
        let bytes = btclisp_codec::encode(&program.core);

        // Golden serialized bytes.
        let expected_hex = fs::read_to_string(dir.join(format!("{stem}.bin.hex")))
            .unwrap_or_else(|_| panic!("missing {stem}.bin.hex"));
        assert_eq!(
            hex_encode(&bytes),
            expected_hex.trim(),
            "{stem}: serialized bytes differ from golden"
        );

        // Optional evaluation check.
        let expected_path = dir.join(format!("{stem}.expected"));
        if expected_path.exists() {
            let spec = fs::read_to_string(&expected_path).expect("read expected");
            check_eval(stem, &program.core, &bytes, &spec);
        }
        checked += 1;
    }

    assert!(
        checked >= 7,
        "expected at least 7 examples, found {checked}"
    );
}

fn check_eval(stem: &str, program: &btclisp_core::CoreExpr, program_bytes: &[u8], spec: &str) {
    let mut env = Value::Nil;
    let mut tx_hex: Option<String> = None;
    let mut expected: Option<String> = None;

    for line in spec.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let value = value.trim();
        match key {
            "env" => {
                let env_program = compile(value).unwrap_or_else(|e| panic!("{stem} env: {e}"));
                let mut ctx = EvalCtx::default();
                env = eval(&env_program.core, &Value::Nil, &mut ctx)
                    .unwrap_or_else(|e| panic!("{stem} env eval: {e}"));
            }
            "tx" => tx_hex = Some(value.to_string()),
            "result" => expected = Some(value.to_string()),
            other => panic!("{stem}: unknown .expected key `{other}`"),
        }
    }

    let expected = expected.unwrap_or_else(|| panic!("{stem}: missing result"));

    let result = match tx_hex {
        Some(hex) => eval_with_tx(program, program_bytes, &env, &hex),
        None => {
            let mut ctx = EvalCtx::default();
            eval(program, &env, &mut ctx)
        }
    };

    if expected == "FAIL" {
        assert!(result.is_err(), "{stem}: expected failure, got {result:?}");
    } else {
        let value = result.unwrap_or_else(|e| panic!("{stem} eval: {e}"));
        assert_eq!(value.to_sexpr(), expected, "{stem}: result mismatch");
    }
}

fn eval_with_tx(
    program: &btclisp_core::CoreExpr,
    program_bytes: &[u8],
    env: &Value,
    tx_hex: &str,
) -> Result<Value, btclisp_vm::EvalError> {
    use bitcoin::taproot::LeafVersion;
    use bitcoin::{ScriptBuf, TapLeafHash, Transaction, XOnlyPublicKey};

    let tx: Transaction = bitcoin::consensus::deserialize(&hex_decode(tx_hex)).expect("decode tx");
    let prevouts: Vec<bitcoin::TxOut> = Vec::new();
    let tapleaf_hash = TapLeafHash::from_script(
        ScriptBuf::from_bytes(program_bytes.to_vec()).as_script(),
        LeafVersion::TapScript,
    );
    let internal_key = XOnlyPublicKey::from_slice(&hex_decode(NUMS_KEY_HEX)).expect("nums key");
    let txc = TxContext {
        tx: &tx,
        input_index: 0,
        prevouts: &prevouts,
        tapleaf_hash,
        internal_key,
        merkle_branch: Vec::new(),
    };
    let mut ctx = EvalCtx::with_tx_context(&txc);
    eval(program, env, &mut ctx)
}
