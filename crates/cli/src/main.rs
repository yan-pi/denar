//! `btclispc` — the btclisp command-line driver.
//!
//! Sprint 1 implements `check` (lex + parse) and `fmt` (canonical
//! pretty-print). The remaining subcommands are stubbed until their sprints.

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
    /// Compile a source file to bytecode (planned: Sprint 3).
    Compile {
        /// Path to a `.btl` source file.
        input: PathBuf,
        /// Output path for the compiled bytes.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Disassemble bytecode (planned: Sprint 8).
    Disasm {
        /// Path to a `.bin` file.
        input: PathBuf,
    },
    /// Run bytecode in the VM (planned: Sprint 4+).
    Run {
        /// Path to a `.bin` file.
        input: PathBuf,
    },
}

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
        Cmd::Compile { .. } | Cmd::Disasm { .. } | Cmd::Run { .. } => {
            anyhow::bail!("subcommand not yet implemented (planned for a later sprint)")
        }
    }
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
