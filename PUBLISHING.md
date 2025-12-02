# Publishing VSF to crates.io

## Quick Command

```bash
cargo publish --no-verify
```

## Why `--no-verify`?

The `build.rs` downloads Huffman frequency tables (`frequencies.bin`, `huffman_codes.bin`)
from GitHub during build. This modifies the source directory, which cargo's verification
step rejects.

The `--no-verify` flag skips this check. The package is still valid - the build.rs will
download the required files when users build the crate.

## Pre-publish Checklist

1. Run tests: `cargo test --lib --features text`
2. Check it builds: `cargo build --release --features text`
3. Bump version in `Cargo.toml`
4. Dry run (optional): `cargo publish --dry-run --allow-dirty --no-verify`
5. Publish: `cargo publish --no-verify`
