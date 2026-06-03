//! Criterion benchmarks for the btclisp pipeline: compile, encode, decode, eval.
//!
//! Run with `cargo bench -p btclisp-vm`.

use std::hint::black_box;

use btclisp_codec::{decode, encode};
use btclisp_compiler::compile;
use btclisp_vm::{eval, EvalCtx, Value};
use criterion::{criterion_group, criterion_main, Criterion};

/// A recursive program that exercises lowering, the codec, and the evaluator.
const FACTORIAL: &str = "(defun fact (n) (if (< n 2) 1 (* n (fact (- n 1)))))\n(fact 10)";

fn pipeline(c: &mut Criterion) {
    let program = compile(FACTORIAL).expect("compile");
    let bytes = encode(&program.core);

    c.bench_function("compile/factorial", |b| {
        b.iter(|| compile(black_box(FACTORIAL)).expect("compile"));
    });

    c.bench_function("encode/factorial", |b| {
        b.iter(|| encode(black_box(&program.core)));
    });

    c.bench_function("decode/factorial", |b| {
        b.iter(|| decode(black_box(&bytes)).expect("decode"));
    });

    c.bench_function("eval/factorial(10)", |b| {
        b.iter(|| {
            let mut ctx = EvalCtx::default();
            eval(black_box(&program.core), &Value::Nil, &mut ctx).expect("eval")
        });
    });

    c.bench_function("pipeline/factorial(10)", |b| {
        b.iter(|| {
            let program = compile(black_box(FACTORIAL)).expect("compile");
            let bytes = encode(&program.core);
            let core = decode(&bytes).expect("decode");
            let mut ctx = EvalCtx::default();
            eval(&core, &Value::Nil, &mut ctx).expect("eval")
        });
    });
}

criterion_group!(benches, pipeline);
criterion_main!(benches);
