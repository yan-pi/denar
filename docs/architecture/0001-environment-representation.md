# 1. Environment representation: flat list (v0,1) → balanced tree (v0,2)

## Status

Accepted for v0,1. The v0,2 env-tree is **Proposed** (scheduled; not yet
implemented).

## Context

Lowering (spec §6) compiles surface `let`/`defun` bodies to core expressions
whose variable references are integer atoms encoding a path into a runtime
environment (Chia/CLVM convention: leading `1` is the terminator, then bits
from the LSB select `head`/`tail`). The `i`-th element of a flat environment
list lives at `param_path(i) = (2 << i) | ((1 << i) - 1)`.

v0,1 deliberately uses a **flat, non-capturing** environment (spec "Non-goals:
Balanced env-tree (use list positions in v0,1; tree in v0,2)"):

- Each `defun`/`let` body is lowered against a fresh `Scope::Closed` containing
  *only* its own bindings; binding values are lowered in the enclosing scope.
- A function call / `let` lowers to `(a (q . BODY) ENV)`, where `ENV` is a flat
  cons list of the evaluated arguments built with `c`.
- Self-recursion (ADR-implied by §14.3) reserves env slot 0 for the program
  itself and shifts params by one.

This is simple and proven (the whole corpus and the regtest harness pass on it),
but it has two consequences:

1. **No closures.** A nested `let`/`defun` body cannot see variables from an
   enclosing scope, because entering the body replaces the environment with the
   new bindings. For example, this is rejected today:

   ```lisp
   (defun f (a) (let ((b 1)) (+ a b)))   ; `a` is not visible in the let body
   ```

2. **Linear paths.** With `n` flat bindings, the deepest reference walks `O(n)`
   `head`/`tail` steps, and each step is a separate atom in the env-building
   expression.

## Decision

**v0,1:** keep the flat list environment exactly as implemented. It satisfies
every v0,1 goal (P2PK, hashlock, CSV-timelock, 2-of-3 multisig, recursion) and
keeps env-ref encoding minimal and auditable.

**v0,2:** replace the flat environment with a **balanced binary tree** that
threads the parent environment, enabling proper lexical closures:

- The environment becomes `(NEW_BINDINGS . PARENT_ENV)`. Entering a scope conses
  the new frame onto the existing environment rather than discarding it, so
  outer variables remain reachable at a deeper path.
- Within a frame, bindings are laid out as a **balanced tree** (built with the
  `b` opcode, byte `0x20`, already implemented in the VM) so that a frame of `k`
  bindings yields `O(log k)` reference paths instead of `O(k)`.
- `resolve::Scope` gains a parent link; `Scope::get` walks the scope chain,
  composing the parent-frame offset with the in-frame tree path to produce the
  final env-ref integer.
- The `a`/`c` call-lowering is generalised: the constructed environment is
  `(b arg0 arg1 ...) . captured_parent` instead of a flat `(c arg0 (c arg1 …))`.

Path arithmetic changes (in-frame tree index + parent depth), so this is the one
change flagged under §15 "pause before altering env-ref encoding" and is staged
as its own effort rather than bundled into v0,1.

## Consequences

**Positive**

- Real closures: nested scopes capture enclosing variables; higher-order
  patterns and `let` over computed values become natural.
- `O(log k)` reference depth per frame; smaller, flatter env-building code for
  wide scopes.
- The `b` opcode finally earns its place in lowering (today it is only reachable
  via the surface operator).

**Negative / risks**

- Env-ref path computation is rewritten; the golden `.bin.hex` vectors for any
  example whose lowering changes must be regenerated.
- Slightly larger environments at runtime (the parent is retained), though
  shared via `Rc` so the cost is a pointer, not a copy.
- Capture analysis is required to avoid retaining unreferenced parent frames if
  we later care about minimality.

**Tracking**

- `crates/compiler/src/lower.rs::tests::nested_scope_captures_outer_variable`
  is `#[ignore]`d and references this ADR; un-ignoring it is the acceptance test
  for the v0,2 rework.
- Benchmarks (`crates/vm/benches/pipeline.rs`) establish the v0,1 baseline so the
  v0,2 change can be measured rather than guessed.
