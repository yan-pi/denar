# Changelog

All notable changes to this project are documented here. The format follows
[Conventional Changelog](https://www.conventionalcommits.org/); this project
adheres to semantic versioning once it reaches 1.0.

## [0.1.0] — v0,1

First feature-complete release against the spec goals: an end-to-end compiler
and eager VM for a reduced subset of ajtowns' BTC Lisp.

### Added

- **Workspace** of five crates: `core`, `compiler`, `codec`, `vm`, `cli`.
- **Frontend** — hand-rolled lexer and recursive-descent parser; surface AST
  with spans and a canonical pretty-printer.
- **Lowering** — name resolution, CLVM env-path arithmetic, `let`/`if`/`cond`
  expansion, `defun` inlining, and direct self-recursion via auto-quine.
- **Codec** — canonical encode/decode for ajtowns' encoding table, with a
  proptest round-trip and an opcode-aware disassembler.
- **Interpreter** — eager `eval` with resource limits; control, list,
  arithmetic, boolean and byte opcodes; `rd`/`wr` serialization.
- **Bitcoin glue** — `sha256`/`hash160`/`hash256`, BIP340 `bip340_verify`, and
  `tx` introspection + `bip342_txmsg` (BIP341 sighash) via `rust-bitcoin`.
- **CLI** `btclispc` — `check`, `fmt`, `compile`, `disasm`, `run` (with optional
  transaction context), and ariadne-rendered diagnostics.
- **Corpus** — examples 01–07 with golden bytes and expected results, plus a
  golden test harness.
- **Regtest** — Nigiri-driven taproot harness behind `--features regtest`.
- **Benchmarks** — criterion pipeline benchmarks.
- **Docs** — README with a Script-vs-btclisp size comparison, a final report,
  and ADR 0001 (environment representation).

### Known limitations

- Non-capturing scopes (closures deferred to v0,2; see ADR 0001).
- Mutual recursion rejected; `u64`-pure integers; `SIGHASH_DEFAULT` only.
- `(tx N)` field codes are a documented v0,1 ABI, pending reconciliation with
  ajtowns' table.
