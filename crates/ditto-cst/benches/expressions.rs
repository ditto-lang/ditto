/*
cargo bench -p ditto-cst > control
cargo bench -p ditto-cst > latest && cargo benchcmp --threshold 5 control latest
*/
#![feature(test)]

extern crate test;

use test::Bencher;

#[bench]
fn bench_qualified_variable(b: &mut Bencher) {
    b.iter(|| ditto_cst::Expression::parse("Foo.bar"));
}

#[bench]
fn bench_long_pipe(b: &mut Bencher) {
    b.iter(|| ditto_cst::Expression::parse("x |> x |> x |> x |> x |> x"));
}

#[bench]
fn bench_very_curried_function(b: &mut Bencher) {
    b.iter(|| ditto_cst::Expression::parse("() -> () -> () -> () -> () -> () -> unit"));
}

#[bench]
fn bench_empty_array(b: &mut Bencher) {
    b.iter(|| ditto_cst::Expression::parse("[]"));
}

#[bench]
fn bench_nested_array(b: &mut Bencher) {
    b.iter(|| ditto_cst::Expression::parse("[[[x]]]"));
}

#[bench]
fn bench_empty_record(b: &mut Bencher) {
    b.iter(|| ditto_cst::Expression::parse("{}"));
}
