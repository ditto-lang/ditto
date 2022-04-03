# Ditto snapshot testing.

Cheapskate data-driven testing for the `ditto-*` crates.

It's not _generally_ useful, yet...

```rust
#[snapshot_test::snapshot_lf(
    input = "golden-tests/(.*).in",
    output = "golden-tests/${1}.out"
)]
fn golden(input: &str) -> String {
  // do stuff with the test `input`
  // return the expected output
}
```
