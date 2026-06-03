//! The eager evaluator (spec §8).

use btclisp_core::{CoreExpr, Opcode};

use crate::error::EvalError;
use crate::limits::{Counters, Limits};
use crate::ops;
use crate::ops::tx::TxContext;
use crate::value::{le_to_u64, Value};

/// Evaluation context: resource budgets, counters, and optional tx context.
#[derive(Clone, Debug, Default)]
pub struct EvalCtx<'a> {
    /// Hard resource budgets.
    pub limits: Limits,
    /// Running resource usage.
    pub counters: Counters,
    /// Transaction-introspection context for `tx`/`bip342_txmsg`.
    pub tx_ctx: Option<&'a TxContext<'a>>,
}

impl<'a> EvalCtx<'a> {
    /// A context with the given limits, zeroed counters, and no tx context.
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            counters: Counters::default(),
            tx_ctx: None,
        }
    }

    /// A context with default limits and the given transaction context.
    #[must_use]
    pub fn with_tx_context(tx_ctx: &'a TxContext<'a>) -> Self {
        Self {
            limits: Limits::default(),
            counters: Counters::default(),
            tx_ctx: Some(tx_ctx),
        }
    }
}

/// Evaluate `expr` against `env`, enforcing the budgets in `ctx`.
///
/// # Errors
///
/// Returns an [`EvalError`] on a program fault (exception, type error, bad
/// path, …) or when a resource budget is exhausted.
///
/// # Example
///
/// ```
/// use btclisp_core::CoreExpr;
/// use btclisp_vm::{eval, EvalCtx, Value};
///
/// // (+ (q . 2) (q . 3)) with the lowered quote shorthand.
/// let program = CoreExpr::cons(
///     CoreExpr::atom(vec![0x17]),
///     CoreExpr::cons(
///         CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x02])),
///         CoreExpr::cons(
///             CoreExpr::cons(CoreExpr::atom(vec![0x00]), CoreExpr::atom(vec![0x03])),
///             CoreExpr::Nil,
///         ),
///     ),
/// );
/// let mut ctx = EvalCtx::default();
/// assert_eq!(eval(&program, &Value::Nil, &mut ctx).unwrap(), Value::from_u64(5));
/// ```
pub fn eval(expr: &CoreExpr, env: &Value, ctx: &mut EvalCtx) -> Result<Value, EvalError> {
    eval_rec(expr, env, ctx, 1)
}

fn eval_rec(
    expr: &CoreExpr,
    env: &Value,
    ctx: &mut EvalCtx,
    depth: u32,
) -> Result<Value, EvalError> {
    if depth > ctx.limits.max_stack {
        return Err(EvalError::StackLimitExceeded);
    }
    ctx.counters.max_depth = ctx.counters.max_depth.max(depth);
    ctx.counters.ops += 1;
    if ctx.counters.ops > ctx.limits.max_ops {
        return Err(EvalError::OpLimitExceeded);
    }

    match expr {
        CoreExpr::Nil => Ok(Value::Nil),
        CoreExpr::Atom(bytes) => lookup_path(bytes, env),
        CoreExpr::Cons(head, tail) => eval_apply(head, tail, env, ctx, depth),
    }
}

/// Evaluate a `(head . tail)` application.
fn eval_apply(
    head: &CoreExpr,
    tail: &CoreExpr,
    env: &Value,
    ctx: &mut EvalCtx,
    depth: u32,
) -> Result<Value, EvalError> {
    let op = operator(head)?;
    match op {
        // `q` returns its argument unevaluated.
        Opcode::Q => Ok(Value::from_core(tail)),
        // `x` aborts the program.
        Opcode::X => Err(EvalError::Exception),
        // `i` is strict in the condition only and short-circuits.
        Opcode::If => {
            let parts = list_elements(tail)?;
            arity(Opcode::If, &parts, 3)?;
            let cond = eval_rec(parts[0], env, ctx, depth + 1)?;
            let branch = if cond.is_truthy() { parts[1] } else { parts[2] };
            eval_rec(branch, env, ctx, depth + 1)
        }
        // `a` evaluates a program in a freshly built environment.
        Opcode::A => {
            let parts = list_elements(tail)?;
            arity(Opcode::A, &parts, 2)?;
            let program = eval_rec(parts[0], env, ctx, depth + 1)?;
            let new_env = eval_rec(parts[1], env, ctx, depth + 1)?;
            eval_rec(&program.to_core(), &new_env, ctx, depth + 1)
        }
        // Everything else is strict: evaluate all arguments, then apply.
        _ => {
            let arg_exprs = list_elements(tail)?;
            let mut args = Vec::with_capacity(arg_exprs.len());
            for arg in arg_exprs {
                args.push(eval_rec(arg, env, ctx, depth + 1)?);
            }
            apply_op(op, &args, ctx.tx_ctx)
        }
    }
}

/// Apply a strict opcode to already-evaluated arguments.
fn apply_op(op: Opcode, args: &[Value], tx_ctx: Option<&TxContext>) -> Result<Value, EvalError> {
    match op {
        Opcode::C => ops::list::cons(args),
        Opcode::H => ops::list::head(args),
        Opcode::T => ops::list::tail(args),
        Opcode::L => ops::list::is_list(args),
        Opcode::BinTree => ops::list::bintree(args),
        Opcode::Not => ops::control::not(args),
        Opcode::All => ops::control::all(args),
        Opcode::Any => ops::control::any(args),
        Opcode::Add => ops::arith::add(args),
        Opcode::Sub => ops::arith::sub(args),
        Opcode::Mul => ops::arith::mul(args),
        Opcode::Mod => ops::arith::rem(args),
        Opcode::DivMod => ops::arith::divmod(args),
        Opcode::Lt => ops::arith::lt(args),
        Opcode::Eq => ops::arith::eq(args),
        Opcode::Cat => ops::bytes::cat(args),
        Opcode::Substr => ops::bytes::substr(args),
        Opcode::Strlen => ops::bytes::strlen(args),
        Opcode::Wr => ops::bytes::write(args),
        Opcode::Rd => ops::bytes::read(args),
        Opcode::Sha256 => ops::hash::sha256(args),
        Opcode::Hash160 => ops::hash::hash160(args),
        Opcode::Hash256 => ops::hash::hash256(args),
        Opcode::Bip340Verify => ops::crypto::bip340_verify(args),
        Opcode::Tx => ops::tx::tx(args, tx_ctx),
        Opcode::Bip342Txmsg => ops::tx::bip342_txmsg(args, tx_ctx),
        // Control opcodes are handled before this point.
        Opcode::Q | Opcode::X | Opcode::If | Opcode::A => {
            Err(EvalError::InvalidOperator(vec![op.as_byte()]))
        }
    }
}

/// Decode the operator atom in head position into an [`Opcode`].
fn operator(head: &CoreExpr) -> Result<Opcode, EvalError> {
    match head {
        CoreExpr::Atom(bytes) if bytes.len() == 1 => {
            Opcode::from_byte(bytes[0]).ok_or_else(|| EvalError::InvalidOperator(bytes.to_vec()))
        }
        CoreExpr::Atom(bytes) => Err(EvalError::InvalidOperator(bytes.to_vec())),
        CoreExpr::Nil => Err(EvalError::InvalidOperator(Vec::new())),
        CoreExpr::Cons(_, _) => Err(EvalError::InvalidOperator(Vec::new())),
    }
}

/// Collect the elements of a proper list, erroring on an improper tail.
fn list_elements(expr: &CoreExpr) -> Result<Vec<&CoreExpr>, EvalError> {
    let mut out = Vec::new();
    let mut cur = expr;
    loop {
        match cur {
            CoreExpr::Cons(head, tail) => {
                out.push(head.as_ref());
                cur = tail;
            }
            CoreExpr::Nil => return Ok(out),
            CoreExpr::Atom(_) => return Err(EvalError::ImproperArgList),
        }
    }
}

fn arity(op: Opcode, parts: &[&CoreExpr], n: usize) -> Result<(), EvalError> {
    if parts.len() == n {
        Ok(())
    } else {
        Err(EvalError::Arity {
            op: op.name(),
            expected: n,
            found: parts.len(),
        })
    }
}

/// Walk an environment along the path encoded by `bytes` (little-endian).
fn lookup_path(bytes: &[u8], env: &Value) -> Result<Value, EvalError> {
    let path = le_to_u64(bytes)?;
    if path == 0 {
        return Ok(Value::Nil);
    }
    let mut node = env.clone();
    let mut remaining = path;
    while remaining > 1 {
        node = match &node {
            Value::Cons(pair) => {
                if remaining & 1 == 1 {
                    pair.1.clone()
                } else {
                    pair.0.clone()
                }
            }
            _ => return Err(EvalError::PathIntoAtom),
        };
        remaining >>= 1;
    }
    Ok(node)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(program: &CoreExpr, env: &Value) -> Result<Value, EvalError> {
        eval(program, env, &mut EvalCtx::default())
    }

    fn q(value: CoreExpr) -> CoreExpr {
        CoreExpr::cons(CoreExpr::atom(vec![0x00]), value)
    }

    fn app(op: u8, args: Vec<CoreExpr>) -> CoreExpr {
        let mut list = CoreExpr::Nil;
        for a in args.into_iter().rev() {
            list = CoreExpr::cons(a, list);
        }
        CoreExpr::cons(CoreExpr::atom(vec![op]), list)
    }

    #[test]
    fn quote_returns_literal() {
        assert_eq!(
            run(&q(CoreExpr::atom(vec![0x07])), &Value::Nil).unwrap(),
            Value::atom(vec![0x07])
        );
    }

    #[test]
    fn addition_evaluates_arguments() {
        let program = app(
            0x17,
            vec![q(CoreExpr::atom(vec![2])), q(CoreExpr::atom(vec![3]))],
        );
        assert_eq!(run(&program, &Value::Nil).unwrap(), Value::from_u64(5));
    }

    #[test]
    fn env_path_lookup() {
        // env = (10 . (20 . nil)); path 2 -> 10, path 5 -> 20.
        let env = Value::cons(
            Value::from_u64(10),
            Value::cons(Value::from_u64(20), Value::Nil),
        );
        assert_eq!(
            run(&CoreExpr::atom(vec![2]), &env).unwrap(),
            Value::from_u64(10)
        );
        assert_eq!(
            run(&CoreExpr::atom(vec![5]), &env).unwrap(),
            Value::from_u64(20)
        );
    }

    #[test]
    fn if_short_circuits() {
        // (i (q.1) (q.42) (x))  -> 42, the else (x) must not run.
        let program = app(
            0x03,
            vec![
                q(CoreExpr::atom(vec![1])),
                q(CoreExpr::atom(vec![42])),
                app(0x02, vec![]),
            ],
        );
        assert_eq!(run(&program, &Value::Nil).unwrap(), Value::from_u64(42));
    }

    #[test]
    fn exception_opcode_fails() {
        assert_eq!(
            run(&app(0x02, vec![]), &Value::Nil),
            Err(EvalError::Exception)
        );
    }

    #[test]
    fn apply_runs_program_in_new_env() {
        // (a (q . 2) (c (q.99) nil)) -> head of new env -> 99
        let body = q(CoreExpr::atom(vec![2]));
        let new_env = app(0x05, vec![q(CoreExpr::atom(vec![99])), q(CoreExpr::Nil)]);
        let program = app(0x01, vec![body, new_env]);
        assert_eq!(run(&program, &Value::Nil).unwrap(), Value::from_u64(99));
    }

    #[test]
    fn op_limit_is_enforced() {
        let mut ctx = EvalCtx::new(Limits {
            max_ops: 1,
            ..Limits::default()
        });
        let program = app(
            0x17,
            vec![q(CoreExpr::atom(vec![1])), q(CoreExpr::atom(vec![1]))],
        );
        assert_eq!(
            eval(&program, &Value::Nil, &mut ctx),
            Err(EvalError::OpLimitExceeded)
        );
    }

    #[test]
    fn tx_without_context_reports_cleanly() {
        assert_eq!(
            run(&app(0x2b, vec![q(CoreExpr::atom(vec![1]))]), &Value::Nil),
            Err(EvalError::NoTxContext)
        );
    }
}
