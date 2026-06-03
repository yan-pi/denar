# btclisp v0,1 — Final Report

A compiler and eager VM for a Lisp-style language targeting a reduced subset of
ajtowns' BTC Lisp, built sprint-by-sprint against [`spec.md`](../spec.md).

## Outcome

All v0,1 spec goals are met:

- End-to-end pipeline `source .btl → core IR → bytes → eager VM result`.
- ~25 core opcodes implemented.
- Bitcoin tx introspection (`tx`) and BIP340 verification operational against
  real transactions; BIP341 sighash via `rust-bitcoin`.
- Corpus covers P2PK, hashlock, CSV-timelock, and 2-of-3 multisig.

**Verification:** 164 tests pass (`fmt` + `clippy --all-targets -D warnings`
clean). The codec round-trips under 2×2048 proptest cases; BIP340/hash opcodes
are checked against reference vectors; a P2TR commitment to a compiled program
is funded and spent on a live regtest node.

## What was built, by sprint

| Sprint | Delivered |
|--------|-----------|
| S1 | Cargo workspace; hand-rolled lexer + recursive-descent parser; AST + canonical pretty-printer; `check`/`fmt`. |
| S2 | Name resolution, function table, CLVM env-path arithmetic; lowering of `let`/`if`/`cond`/calls; `defun` inlining; snapshot tests. |
| S3 | Canonical binary codec (ajtowns' table) with proptest round-trip; `compile`/`disasm` wiring. |
| S4 | Eager interpreter (`eval`): control flow, arithmetic, list and byte ops; resource limits; `run`. |
| S5 | `sha256`/`hash160`/`hash256`; offline `bip340_verify` (BIP340 vectors). |
| S6 | `tx` introspection + `bip342_txmsg` (BIP341 sighash) via `rust-bitcoin`; `TxContext`; real taproot P2PK sign+verify. |
| S7 | Examples 01–07 with golden bytes and expected results; golden harness; auto-quine self-recursion (factorial). |
| S8 | Opcode-aware disassembler; ariadne-rendered diagnostics. |
| S9 | Nigiri regtest harness (`--features regtest`): taproot commit + fund + key-path spend + off-chain script validation. |
| S10 | Criterion pipeline benchmarks; ADR 0001 for the v0,2 env-tree; ignored acceptance test for closures. |
| S11 | README, size comparison, this report, changelog. |

## Key decisions

These were genuine forks in the road, resolved and documented as they came up:

- **Hand-rolled parser** instead of the locked `chumsky` — Lisp's grammar is
  trivial, and owning the lexer/parser gives precise spans for TDD. ariadne
  still renders diagnostics on top of those spans (S8).
- **Codec under-specified codes** (`0x34`, `0x7f`, varints) — implemented
  faithfully where the §7 table is unambiguous, with documented choices
  (length-prefixed `0x34`, `LEB128(size<<1|proper)` for `0x7f`, unsigned LEB128)
  elsewhere. Canonical encoding makes `decode∘encode = id`.
- **Core `a` is 2-arg**, `if` lowers to `(a (i C (q.T) (q.E)) 1)`, and `cond`
  falls through to `(x)`.
- **Flat, non-capturing scopes** for v0,1 (spec's explicit simplification); the
  balanced env-tree for closures is specified in ADR 0001 and scheduled for
  v0,2.
- **Auto-quine self-recursion** (resolves spec open question §14.3): a recursive
  function receives itself at env slot 0 and recurses through an explicit `a`.
- **`u64`-pure arithmetic** (§14.1): `+`/`*` wrap, `-` saturates.
- **`(tx N)` field codes** — a documented v0,1 ABI (user-approved), to reconcile
  with ajtowns' exact table later.

## The regtest honesty note

bitcoind cannot execute btclisp: it is a *proposed* opcode set, not a deployed
softfork. A script-path spend revealing a btclisp tapleaf would be rejected
(leaf version `0xc0` runs as BIP342 tapscript). The regtest harness therefore
proves the realistic thing: a real P2TR output **committing** to a compiled
program is funded and spent (via the key path), while the **script-path
semantics** are enforced off-chain by our interpreter — exactly what a
btclisp-aware validator would do.

## Deferred to v0,2 (and beyond)

- Balanced env-tree + capturing closures (ADR 0001); mutual recursion.
- CScriptNum integer semantics; the `partial` and softfork (`sf`) opcodes.
- Lazy evaluation; a static/gradual type system.
- Consensus-grade costing (only rough resource counters exist today).
- Golden cross-check against ajtowns' published serialization vectors and exact
  `tx` field-code table.

## Baselines

From `cargo bench -p btclisp-vm --bench pipeline` (factorial, release):
compile ≈ 6.9 µs, encode ≈ 0.94 µs, decode ≈ 3.4 µs, `eval fact(10)` ≈ 25.6 µs.
These establish a reference point for measuring the v0,2 env-tree change.
