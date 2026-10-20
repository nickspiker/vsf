# Publishing VSF to crates.io

## Order

`vsf` depends on `vsf-derive` (this repo's `derive/`), so the derive crate goes first:

```bash
cd derive && cargo publish      # vsf-derive
cd .. && cargo publish          # vsf
```

The format version is the crate's breaking number (`VSF_VERSION` must equal it, enforced at compile time in `src/lib.rs`): vsf 0.10.x is format v10. When the release goes out, give its `CHANGELOG.md` heading a date.

## Path dependencies

Sibling crates are declared with both a version and a path, for example:

```toml
spirix = { version = "0.1.1", path = "../spirix", optional = true }
```

Keep both. Locally the path wins, so the sibling checkout is the one compiled. At publish, Cargo strips the path and the published crate depends on the version from crates.io. A dependency with a path and no version is refused at publish, so every non-dev dependency needs its version.

## Verification

`build.rs` reads the vendored codebook (`huffman_codes.bin`, in the package) and checks it and the corpus (`frequencies.bin`, excluded from the package to stay under the 10 MiB limit) against pinned BLAKE3 hashes. It downloads nothing and writes nothing into the source tree, so the normal verified `cargo publish` should work. If verification ever fails for an environmental reason, `cargo publish --no-verify` is the fallback; run the tests first.

## Checklist

0. `cargo test` and `cargo build --all-features --release`.
1. Bump the version in `Cargo.toml` (and `VSF_VERSION` with it when the breaking number changes).
2. `cargo publish --dry-run` here and in `derive/` (vsf's dry run needs `vsf-derive` already on crates.io, because a dry run resolves dependencies from the registry).
3. Publish `vsf-derive`, then `vsf`.

## The sibling crates

Crates that depend on vsf (manifestus, tohu, kete, toka, fluor, fgtw, limbus, opsin) publish after it, each after the siblings it depends on. Their dry runs fail with "failed to select a version" until their dependencies are on crates.io; that is ordering, not a fault. The order for the v10 release:

0. `spirix` (no sibling dependencies)
1. `vsf-derive`, then `vsf`
2. `manifestus`, `tohu`, `fluor`, `fgtw`, `limbus`
3. `kete` (needs manifestus, tohu), `toka` (needs fluor)
4. `opsin` (needs fluor, limbus)
