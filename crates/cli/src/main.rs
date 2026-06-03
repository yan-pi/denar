//! `btclispc` — the btclisp command-line driver.
//!
//! Implements `check`, `fmt`, `compile`, `disasm`, and `run` (with optional
//! transaction context for `tx`/`bip342_txmsg`).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use btclisp_compiler::{parse_program, Error};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "btclispc", version, about = "btclisp compiler & VM")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Lex, parse, and resolve a source file, reporting any errors.
    Check {
        /// Path to a `.btl` source file.
        input: PathBuf,
        /// Also print the lowered core expression as an s-expression.
        #[arg(long)]
        core: bool,
    },
    /// Canonically format (pretty-print) a source file.
    Fmt {
        /// Path to a `.btl` source file.
        input: PathBuf,
        /// Rewrite the file in place instead of printing to stdout.
        #[arg(long)]
        in_place: bool,
    },
    /// Compile a source file to bytecode (writes raw bytes, or hex to stdout).
    Compile {
        /// Path to a `.btl` source file.
        input: PathBuf,
        /// Output path for the compiled bytes.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Disassemble bytecode back to an s-expression.
    Disasm {
        /// Path to a `.bin` file.
        input: PathBuf,
    },
    /// Run compiled bytecode in the interpreter.
    Run {
        /// Path to a `.bin` file.
        input: PathBuf,
        /// Environment as the hex of an encoded core expression (default nil).
        #[arg(long)]
        env: Option<String>,
        /// Spending transaction as raw consensus hex (enables `tx`/`bip342_txmsg`).
        #[arg(long)]
        tx: Option<String>,
        /// Prevouts as JSON: `[{"value": sats, "script_pubkey": "hex"}, ...]`.
        #[arg(long)]
        prevouts: Option<PathBuf>,
        /// Index of the input being validated.
        #[arg(long, default_value_t = 0)]
        input_index: u32,
    },
}

/// BIP341 unspendable NUMS internal key, used as the default internal key.
const NUMS_KEY_HEX: &str = "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Check { input, core } => cmd_check(&input, core),
        Cmd::Fmt { input, in_place } => cmd_fmt(&input, in_place),
        Cmd::Compile { input, output } => cmd_compile(&input, output.as_deref()),
        Cmd::Disasm { input } => cmd_disasm(&input),
        Cmd::Run {
            input,
            env,
            tx,
            prevouts,
            input_index,
        } => cmd_run(
            &input,
            env.as_deref(),
            tx.as_deref(),
            prevouts.as_deref(),
            input_index,
        ),
    }
}

fn cmd_run(
    path: &Path,
    env_hex: Option<&str>,
    tx_hex: Option<&str>,
    prevouts_path: Option<&Path>,
    input_index: u32,
) -> Result<()> {
    use bitcoin::taproot::LeafVersion;
    use bitcoin::{ScriptBuf, TapLeafHash, Transaction, XOnlyPublicKey};

    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let program = btclisp_codec::decode(&bytes)
        .with_context(|| format!("decoding program {}", path.display()))?;

    let env = match env_hex {
        Some(hex) => {
            let env_bytes = from_hex(hex).context("parsing --env hex")?;
            let env_expr = btclisp_codec::decode(&env_bytes).context("decoding --env")?;
            btclisp_vm::Value::from_core(&env_expr)
        }
        None => btclisp_vm::Value::Nil,
    };

    // Build a transaction context if a spending transaction was supplied.
    let parsed_tx: Option<Transaction> = match tx_hex {
        Some(hex) => {
            let tx_bytes = from_hex(hex).context("parsing --tx hex")?;
            Some(bitcoin::consensus::deserialize(&tx_bytes).context("decoding --tx")?)
        }
        None => None,
    };
    let prevouts = match prevouts_path {
        Some(p) => parse_prevouts(p)?,
        None => Vec::new(),
    };

    let tx_ctx = match parsed_tx.as_ref() {
        Some(tx) => {
            let tapleaf_hash = TapLeafHash::from_script(
                ScriptBuf::from_bytes(bytes.clone()).as_script(),
                LeafVersion::TapScript,
            );
            let internal_key = XOnlyPublicKey::from_slice(&from_hex(NUMS_KEY_HEX)?)
                .context("parsing NUMS internal key")?;
            Some(btclisp_vm::TxContext {
                tx,
                input_index,
                prevouts: &prevouts,
                tapleaf_hash,
                internal_key,
                merkle_branch: Vec::new(),
            })
        }
        None => None,
    };

    let mut ctx = match tx_ctx.as_ref() {
        Some(tc) => btclisp_vm::EvalCtx::with_tx_context(tc),
        None => btclisp_vm::EvalCtx::default(),
    };
    let result = btclisp_vm::eval(&program, &env, &mut ctx)
        .map_err(|e| anyhow::anyhow!("evaluation failed: {e}"))?;
    println!("{}", result.to_sexpr());
    eprintln!("({} op(s))", ctx.counters.ops);
    Ok(())
}

fn parse_prevouts(path: &Path) -> Result<Vec<bitcoin::TxOut>> {
    use bitcoin::{Amount, ScriptBuf, TxOut};

    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading prevouts {}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&text).context("parsing prevouts JSON")?;
    let array = json
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("prevouts JSON must be an array"))?;

    let mut prevouts = Vec::with_capacity(array.len());
    for entry in array {
        let value = entry["value"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("prevout `value` must be an integer (sats)"))?;
        let spk_hex = entry["script_pubkey"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("prevout `script_pubkey` must be a hex string"))?;
        prevouts.push(TxOut {
            value: Amount::from_sat(value),
            script_pubkey: ScriptBuf::from_bytes(from_hex(spk_hex)?),
        });
    }
    Ok(prevouts)
}

fn from_hex(s: &str) -> Result<Vec<u8>> {
    let trimmed = s.trim();
    if trimmed.len() % 2 != 0 {
        anyhow::bail!("hex string has an odd number of digits");
    }
    (0..trimmed.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&trimmed[i..i + 2], 16).context("invalid hex digit"))
        .collect()
}

fn cmd_compile(path: &Path, output: Option<&Path>) -> Result<()> {
    let src = read_source(path)?;
    let program = match btclisp_compiler::compile(&src) {
        Ok(program) => program,
        Err(err) => {
            print_diagnostic(path, &src, &err);
            anyhow::bail!("compilation failed");
        }
    };
    let bytes = btclisp_codec::encode(&program.core);
    match output {
        Some(out) => {
            std::fs::write(out, &bytes).with_context(|| format!("writing {}", out.display()))?;
            eprintln!("✓ wrote {} byte(s) to {}", bytes.len(), out.display());
        }
        None => println!("{}", to_hex(&bytes)),
    }
    Ok(())
}

fn cmd_disasm(path: &Path) -> Result<()> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let expr =
        btclisp_codec::decode(&bytes).with_context(|| format!("decoding {}", path.display()))?;
    println!("{}", expr.to_sexpr());
    Ok(())
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

fn cmd_check(path: &Path, show_core: bool) -> Result<()> {
    let src = read_source(path)?;
    let forms = match parse_program(&src) {
        Ok(forms) => forms,
        Err(err) => {
            print_diagnostic(path, &src, &err);
            anyhow::bail!("check failed");
        }
    };
    let program = match btclisp_compiler::lower_program(&forms) {
        Ok(program) => program,
        Err(err) => {
            print_diagnostic(path, &src, &err);
            anyhow::bail!("check failed");
        }
    };

    let env = if program.env_params.is_empty() {
        "(none)".to_string()
    } else {
        program.env_params.join(" ")
    };
    println!(
        "✓ {}: {} form(s) parsed, lowered to core; env: {env}",
        path.display(),
        forms.len()
    );
    if show_core {
        println!("{}", program.core.to_sexpr());
    }
    Ok(())
}

fn cmd_fmt(path: &Path, in_place: bool) -> Result<()> {
    let src = read_source(path)?;
    let forms = match parse_program(&src) {
        Ok(forms) => forms,
        Err(err) => {
            print_diagnostic(path, &src, &err);
            anyhow::bail!("formatting failed: source does not parse");
        }
    };

    let mut out = String::new();
    for form in &forms {
        out.push_str(&form.to_string());
        out.push('\n');
    }

    if in_place {
        std::fs::write(path, out).with_context(|| format!("writing {}", path.display()))?;
    } else {
        print!("{out}");
    }
    Ok(())
}

fn read_source(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// Print a compact `path:line:col` diagnostic with a caret under the span.
fn print_diagnostic(path: &Path, src: &str, err: &Error) {
    match err.span() {
        Some(span) => {
            let (line, col) = line_col(src, span.start);
            eprintln!("{}:{}:{}: error: {}", path.display(), line, col, err);
            if let Some(text) = src.lines().nth(line - 1) {
                eprintln!("  {line:>4} | {text}");
                let pad = " ".repeat(col.saturating_sub(1));
                eprintln!("       | {pad}^");
            }
        }
        None => eprintln!("{}: error: {}", path.display(), err),
    }
}

/// Convert a byte offset into a 1-based `(line, column)`.
fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in src.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}
