mod common;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    let source = include_str!("vdom-html/source.ditto");
    let (cst_module, ast_module, everything, codegen_js_config) = common::preamble(
        source,
        vec![include_str!("vdom-html/Attributes.ast-exports")],
    );
    let mut group = c.benchmark_group("vdom:Html");
    group.bench_function("parse", |b| {
        b.iter(|| ditto_cst::Module::parse(black_box(source)).unwrap())
    });
    group.bench_function("check", |b| {
        b.iter_batched(
            || cst_module.clone(),
            |cst_module| ditto_checker::check_module(black_box(&everything), black_box(cst_module)),
            BatchSize::LargeInput,
        )
    });
    group.bench_function("codegen-js", |b| {
        b.iter_batched(
            || ast_module.clone(),
            |ast_module| ditto_codegen_js::codegen(&codegen_js_config, ast_module),
            BatchSize::LargeInput,
        )
    });
    group.finish()
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(30);
    targets = criterion_benchmark
}
criterion_main!(benches);
