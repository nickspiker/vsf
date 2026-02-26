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

0. Run tests: `cargo test --lib --features text`
1. Check it builds: `cargo build --release --features text`
2. Bump version in `Cargo.toml`
3. **If spirix changed:** remove `path = "../spirix"` from the spirix dep in `Cargo.toml` before publishing (crates.io doesn't allow path deps), then restore it after.
4. Dry run (optional): `cargo publish --dry-run --allow-dirty --no-verify`
5. Publish: `cargo publish --no-verify`
6. Restore `path = "../spirix"` in `Cargo.toml`

## Sibling Crate Note

During local development, `spirix` is declared with a `path` override:

```toml
spirix = { version = "0.0.7", path = "../spirix", optional = true }
```

The `path` must be **removed** before publishing (crates.io rejects path dependencies), but
must be **kept during development** — without it, Cargo resolves spirix from crates.io, which
can diverge from the local copy and cause type identity errors even at the same version string.
