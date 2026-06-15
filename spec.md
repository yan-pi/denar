# btclisp — Spec v0,1

**Edit token**: `BTL01A`

Compiler + VM for a Lisp-style high-level language targeting a reduced subset of ajtowns' BTC Lisp (Delving Bitcoin #682). Written in Rust. Designed to run offline and against `bitcoind` regtest via `rust-bitcoin`.

---

## 0. Goals / non-goals

**Goals (v0,1)**
- End-to-end pipeline: source `.btl` → core IR → bytes → eager VM result.
- ~25 core opcodes (subset of ajtowns' table).
- Bitcoin tx introspection (`tx`) and BIP340 verify operational against real txs.
- Test corpus covers P2PK, hashlock, CSV-timelock, 2-of-3 multisig.

**Non-goals (deferred)**
- Lazy evaluation (eager only).
- `partial` opcode.
- Static / gradual type system (untyped initially).
- Balanced env-tree (use list positions in v0,1; tree in v0,2).
- Softfork opcode (`sf`).
- Consensus-grade costing (rough resource counters only).

---

## 1. Checklist (build order)

1. Cargo workspace: `core`, `compiler`, `codec`, `vm`, `cli`.
2. Surface grammar (EBNF) + lexer + parser → `Form` / `Expr` AST.
3. Core IR (`CoreExpr` = atoms + cons + numeric env refs; opcode enum).
4. Lowering passes: name resolution, `defun`/`let` expansion, env-ref assignment.
5. Binary codec (ajtowns' encoding table). Round-trip property tests.
6. Interpreter: `eval(CoreExpr, Value) -> Result<Value, EvalError>`.
7. Bitcoin glue: `bip340_verify` via `secp256k1`; `tx`/`bip342_txmsg` via `rust-bitcoin`.
8. CLI (`btclispc`) + examples + golden tests + regtest harness.

---

## 2. Workspace layout

```
btclisp/
├── Cargo.toml                      # [workspace] resolver = "2"
├── rust-toolchain.toml             # channel = "stable"
├── SPEC.md                         # this file
├── crates/
│   ├── core/                       # IR types, opcode enum, error types
│   │   └── src/{lib,ir,opcode,error}.rs
│   ├── compiler/                   # source → CoreExpr
│   │   └── src/{lib,lex,parse,ast,resolve,lower,error}.rs
│   ├── codec/                      # CoreExpr ↔ bytes
│   │   └── src/{lib,encode,decode,error}.rs
│   ├── vm/                         # eager interpreter + bitcoin glue
│   │   └── src/{lib,value,env,eval,ops/{control,list,arith,bytes,hash,crypto,tx},error,limits}.rs
│   └── cli/                        # btclispc binary
│       └── src/main.rs
├── examples/                       # .btl programs + .expected + .bin.hex
└── tests/                          # workspace-level integration suites
```

**Crate dependencies (no cycles)**: `cli → {compiler, codec, vm}`; `compiler → core`; `codec → core`; `vm → {core, codec}`.

---

## 3. Surface grammar (EBNF)

```
program     ::= form+
form        ::= defun | expr
defun       ::= '(' 'defun' ident '(' ident* ')' expr ')'
expr        ::= atom
              | "'" expr                          ; quote sugar for (q . expr)
              | '(' 'let'  '(' binding+ ')' expr+ ')'
              | '(' 'if'   expr expr expr ')'
              | '(' 'cond' clause+ ')'
              | '(' head expr* ')'
binding     ::= '(' ident expr ')'
clause      ::= '(' expr expr ')'
head        ::= core_op | ident
atom        ::= integer | hex_lit | string_lit | ident | 'nil'
integer     ::= '-'? [0-9]+                       ; encoded LE, two's complement when negative
hex_lit     ::= '0x' [0-9a-fA-F]+                 ; raw bytes
string_lit  ::= '"' utf8_char* '"'                ; bytes = utf-8 encoding
ident       ::= [a-zA-Z_][a-zA-Z0-9_\-?!]*
core_op     ::= "q"|"a"|"x"|"if"|"c"|"h"|"t"|"l"|"b"
              | "="|"<"|"not"|"all"|"any"
              | "+"|"-"|"*"|"/"|"%"
              | "cat"|"substr"|"strlen"
              | "sha256"|"hash160"|"hash256"
              | "bip340_verify"|"tx"|"bip342_txmsg"
              | "rd"|"wr"
```

Comments: `;` to EOL. Whitespace: spaces, tabs, newlines.

---

## 4. Core opcodes (v0,1 subset)

Byte values from ajtowns' table; all others reserved.

| Opcode             | Byte | Arity     | Notes                                                        |
|--------------------|------|-----------|--------------------------------------------------------------|
| `q` quote          | 0x00 | special   | returns args unevaluated                                     |
| `a` apply          | 0x01 | (P ENV…)  | builds env from ENV…, evaluates P in it                      |
| `x` exception      | 0x02 | (…)       | fails the program                                            |
| `i` if             | 0x03 | (C T E)   | strict in C only; eager interpreter must short-circuit       |
| `c` cons           | 0x05 | (H T)     |                                                              |
| `h` head           | 0x06 | (L)       | error if L is atom                                           |
| `t` tail           | 0x07 | (L)       | error if L is atom                                           |
| `l` list?          | 0x08 | (X)       |                                                              |
| `b` bintree        | 0x20 | (…)       | balanced cons of args (v0,2 use)                             |
| `not`              | 0x09 | (…)       |                                                              |
| `all`              | 0x0a | (…)       |                                                              |
| `any`              | 0x0b | (…)       |                                                              |
| `=`                | 0x0c | (A B …)   |                                                              |
| `<` lt_le          | 0x1e | (A B …)   | atoms as unsigned LE                                         |
| `strlen`           | 0x0e | (A B …)   | sum of lengths                                               |
| `substr`           | 0x0f | (A B E)   |                                                              |
| `cat`              | 0x10 | (A B …)   |                                                              |
| `+`                | 0x17 | (A B …)   | u64 LE                                                       |
| `-`                | 0x18 | (A B …)   | u64 LE                                                       |
| `*`                | 0x19 | (A B …)   | u64 LE                                                       |
| `/`                | —    | (A B)     | exposed via `divmod` (`/%` 0x1b) → `(h (/% a b))`            |
| `%`                | 0x1a | (A B)     |                                                              |
| `sha256`           | 0x24 | (A B …)   | concat then hash                                             |
| `hash160`          | 0x26 | (A B …)   | ripemd160(sha256(concat))                                    |
| `hash256`          | 0x27 | (A B …)   | sha256(sha256(concat))                                       |
| `bip340_verify`    | 0x28 | (K M S)   | nil S → 0; failed verify → script fail                       |
| `tx`               | 0x2b | (…)       | see §8                                                       |
| `bip342_txmsg`     | 0x2c | (SH?)     |                                                              |
| `rd`               | 0x22 | (A)       | bytes → expression                                           |
| `wr`               | 0x23 | (A)       | expression → bytes                                           |

---

## 5. IR types (Rust)

```rust
// crates/core/src/ir.rs
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreExpr {
    Nil,
    Atom(bytes::Bytes),
    Cons(Box<CoreExpr>, Box<CoreExpr>),
}

// crates/core/src/opcode.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Opcode { Q = 0x00, A = 0x01, X = 0x02, If = 0x03, /* … */ }
impl Opcode {
    pub fn from_byte(b: u8) -> Option<Self>;
    pub fn as_byte(self) -> u8;
    pub fn name(self) -> &'static str;
}
```

Surface AST (`compiler::ast`) keeps source spans for diagnostics. `CoreExpr` does not.

---

## 6. Lowering (v0,1: list-based env)

For `(defun f (a b c) body)` the env passed to `f` is `(a . (b . (c . nil)))`. Variable refs are integer atoms representing the env path:

- bit pattern `1` ends the walk
- `0` = head, `1` = tail (matching Chia)
- `a` → path `h env` → ref `2` (`0b10`)
- `b` → path `h (t env)` → ref `5` (`0b101`)
- `c` → path `h (t (t env))` → ref `11` (`0b1011`)

Encoded into `CoreExpr::Atom(le_bytes(ref))`. Treated by interpreter as env lookup when in operator position with no opcode opcode tag (i.e. an integer atom in operator position = env ref, per ajtowns).

**Passes** (in `compiler::lower`):
1. `collect_defuns`: gather all `defun` top-levels into a `FunctionTable`.
2. `resolve`: walk expr tree; map idents → either env-ref path or function id; produce errors for undefined idents.
3. `expand_let`: `(let ((x e1) (y e2)) body)` → `(a (q . body') ((q . e1) (q . e2)))` with body' lowered using x→2, y→5.
4. `expand_if`: surface `if` → `(a (i C (q . T) (q . E)))` so non-taken branch isn't evaluated.
5. `inline_calls`: for v0,1, monomorphize function calls (inline body with env built from args). Recursive functions require explicit `a` self-reference; provide stdlib helpers in `examples/lib.btl`.
6. `emit`: build `CoreExpr`.

V0,2 will replace step 5 with proper closure construction via env-tree (`b` opcode).

---

## 7. Binary codec

Implement ajtowns' table verbatim. Key encoding rules:

| Byte range  | Meaning                                                                 |
|-------------|-------------------------------------------------------------------------|
| `0x00`      | nil                                                                     |
| `0x01–0x33` | single-byte atom with that value                                        |
| `0x34`      | "leftover" single-byte atom or multi-byte atom of length 65–97          |
| `0x35–0x73` | multi-byte atom, length 2–64 follows                                    |
| `0x74`      | varint length, multi-byte atom of length ≥ 98                           |
| `0x75–0x79` | nil-terminated list, 1–5 entries follow                                 |
| `0x7a–0x7e` | improper list, 2–6 entries (last = terminator)                          |
| `0x7f`      | long list: varint header (size + proper flag), then entries             |
| `0x80–0xff` | same as `0x00–0x7f` but quoted (i.e. cons of `q` and the value)         |

**Tests** (`codec/tests/roundtrip.rs`):
- Property: `decode(encode(e)) == e` for arbitrary `CoreExpr` (use `proptest`).
- Property: `encode(decode(b)).is_ok_and(|b2| b2 == b)` for valid `b`.
- Golden vectors from BTC Lisp post (Schnorr verify program, BIP342 sighash program).

---

## 8. Interpreter

`vm::eval(expr: &CoreExpr, env: &Value, ctx: &mut EvalCtx) -> Result<Value, EvalError>`.

```rust
// crates/vm/src/value.rs
#[derive(Clone, Debug)]
pub enum Value {
    Nil,
    Atom(bytes::Bytes),
    Cons(std::rc::Rc<(Value, Value)>),
}

// crates/vm/src/eval.rs
pub struct EvalCtx<'a> {
    pub tx_ctx: Option<&'a TxContext<'a>>,
    pub limits: Limits,
    pub counters: Counters,                  // ops, bytes_alloc, stack_depth
}
```

Dispatch:
1. If `expr` is `Nil` → `Value::Nil`.
2. If `expr` is `Atom(bytes)`:
   - Operator position: treat as env-ref path; walk `env`.
   - Otherwise: literal atom.
3. If `expr` is `Cons(head, tail)`:
   - Evaluate `head` to determine opcode (must be atom of length 1 mapping to `Opcode`, or `q`'s special form).
   - For each opcode, evaluate args left-to-right with the appropriate handler.

**`q` and `i` special-case**: handler receives raw (unevaluated) args. All others get pre-evaluated args.

**Resource limits**:
```rust
pub struct Limits { pub max_ops: u64, pub max_alloc_bytes: u64, pub max_stack: u32 }
impl Default for Limits { fn default() -> Self { Self { max_ops: 1_000_000, max_alloc_bytes: 5_000_000, max_stack: 1_000 } } }
```

**`tx` opcode**:
```rust
// crates/vm/src/ops/tx.rs
pub struct TxContext<'a> {
    pub tx: &'a bitcoin::Transaction,
    pub input_index: u32,
    pub prevouts: &'a [bitcoin::TxOut],
    pub tapleaf_hash: bitcoin::TapLeafHash,
    pub internal_key: bitcoin::XOnlyPublicKey,
    pub merkle_branch: Vec<[u8; 32]>,
}
```

Fields exposed match ajtowns' table §Transaction Introspection. `bip342_txmsg` reuses `bitcoin::sighash::SighashCache` for the BIP341 sighash, restricted to SIGHASH_DEFAULT initially.

---

## 9. CLI (`btclispc`)

```
btclispc compile <input.btl> [-o <out.bin>]
btclispc disasm  <input.bin>
btclispc run     <input.bin> [--env <env.expr>] [--tx <hex>] [--prevouts <prevouts.json>] [--input-index N]
btclispc fmt     <input.btl> [--in-place]
btclispc check   <input.btl>                ; lex + parse + resolve, no codegen
```

`--tx` accepts raw hex; `--prevouts` accepts JSON `[{"value": sats, "script_pubkey": "hex"}, …]`.

---

## 10. Test corpus

`examples/`:

| File                     | Demonstrates                                                  |
|--------------------------|---------------------------------------------------------------|
| `01_add.btl`             | `(+ 2 3)` → `5`                                               |
| `02_factorial.btl`       | recursion via explicit `a` self-reference                     |
| `03_p2pk.btl`            | `(bip340_verify pk (bip342_txmsg) sig)`                       |
| `04_hashlock.btl`        | `(= (sha256 preimage) target_hash)`                           |
| `05_csv_timelock.btl`    | `(< deadline (tx 1))`                                         |
| `06_multisig_2of3.btl`   | three verifies + threshold check                              |
| `07_pretty_print.btl`    | demonstrates `rd`/`wr` round-trip                             |

Each example has:
- `examples/NN_name.btl` — source
- `examples/NN_name.bin.hex` — golden serialized bytes
- `examples/NN_name.expected` — env (optional) + expected eval result or `FAIL`

Integration test in `tests/golden.rs` runs the full pipeline for each example.

Regtest harness in `tests/regtest.rs` (gated behind `--features regtest`):
- Spins up `bitcoind -regtest` via `bitcoind` crate.
- Builds Taproot output committing to a `btclispc`-compiled script.
- Funds + spends; asserts `sendrawtransaction` succeeds.

---

## 11. Dependencies (locked)

```toml
# crates/vm/Cargo.toml
[dependencies]
secp256k1   = { version = "0.29", features = ["recovery"] }
bitcoin     = { version = "0.32", features = ["serde"] }
sha2        = "0.10"
ripemd      = "0.1"
bytes       = "1"
thiserror   = "1"

# crates/compiler/Cargo.toml
[dependencies]
chumsky     = "0.9"
ariadne     = "0.4"                          ; diagnostics
thiserror   = "1"

# dev-deps for codec
proptest    = "1"
```

---

## 12. Conventions

- Toolchain: stable.
- `rustfmt` (default config) + `clippy --all-targets -- -D warnings -W clippy::pedantic`.
- `thiserror` for typed errors; no `anyhow` in library crates. `anyhow` allowed only in `cli`.
- No `unwrap` / `expect` outside tests and `cli::main`.
- One pub `Error` enum per crate (`core::Error`, `compiler::Error`, etc.).
- All public APIs have doc comments with at least one example.
- Every public function has at least one unit test.
- No `unsafe`.
- MSRV: 1,80,0.

---

## 13. Roadmap

| Sprint | Horas | Entregas                                                                  |
|--------|-------|---------------------------------------------------------------------------|
| S1     | 12,0  | Workspace, lexer, parser, AST pretty-printer, `check` subcommand          |
| S2     | 14,0  | `resolve` + `lower` (list-based env), defun inlining, snapshot tests      |
| S3     | 10,0  | `CoreExpr` + codec encode/decode + proptest round-trip                    |
| S4     | 16,0  | Interpreter: control flow, arith, list ops, bytes ops                     |
| S5     | 12,0  | `sha256`/`hash160`/`hash256` + `bip340_verify` (offline)                  |
| S6     | 18,0  | `tx` + `bip342_txmsg` + `rust-bitcoin` integration                        |
| S7     | 12,0  | Examples 01–07 + golden tests                                             |
| S8     | 10,0  | CLI polish, `fmt`, `disasm`, error UX via `ariadne`                       |
| S9     | 14,0  | Regtest harness, end-to-end Taproot spend                                 |
| S10    | 12,0  | Env-tree v0,2 (`b` opcode) + benchmarks                                   |
| S11    | 10,0  | README, comparison table (Script vs btclisp sizes), relatório final      |

**Total estimado**: 140,0 h.

---

## 14. Open questions (decidir antes de S6)

1. Negative integers: CScriptNum-compatible little-endian-signed, ou u64 puro? Recomendação: u64 puro em v0,1; CScriptNum em v0,2.
2. `tx` opcode quando `tx_ctx` é `None`: erro ou nil? Recomendação: `EvalError::NoTxContext`.
3. Recursão na linguagem de superfície: aceitar `(defun f … (f …))` via auto-quine? Ou exigir `a` explícito? Recomendação: quine automático no lowering (mais ergonômico para banca).
4. Codec endianness do varint em `0x74` e `0x7f`: little-endian base-128 (LEB128) ou big-endian? Recomendação: LEB128 unsigned.

