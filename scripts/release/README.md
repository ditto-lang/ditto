# Release generator

This script is responsible for generating GitHub release artifacts.

```sh
# Build the ditto executable
cargo build --release --locked

# Prepare the release
node scripts/release \
  --ditto-bin target/release/ditto \
  --out-zip ditto-linux.zip \
  --out-sha256 ditto-linux.sha256
```
