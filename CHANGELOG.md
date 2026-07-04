# Changelog

All notable changes to VSF will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2026-07-04

### Changed (breaking)

- **Spirix dependency bumped to 0.1.1** (from 0.0.12). Spirix 0.1 changes the Scalar binary representation (implicit sign bit, AMBIG=0 exponent convention), so any VSF file containing Spirix scalar payloads written under 0.0.x will decode to DIFFERENT values under 0.9.0. Circle payloads are unchanged. Spirix 0.1.1 also fixes panics reachable from ordinary arithmetic in debug builds, one of which VSF's own colour conversion tests exposed.

### Fixed

- `BitPackedTensor` is now re-exported at the crate root alongside its tensor siblings (its own doc example imported it from there).
- Repaired six broken doc examples: an unbalanced code fence in the crate docs swallowed the Quick Start section, method chains had been folded into line comments, and two examples referenced pre-refactor APIs (`schema.builder()`/`add_field`/`build` instead of `build()`/`field()`/`encode()`).
- `rand` and `aes-gcm` optional dependencies gained the `getrandom`/`std_rng` features their code paths need, so `--all-features` builds again (rand's `OsRng`/`thread_rng` are feature-gated).

### Known issues

- `examples/show_shortcode_colours.rs` (behind the `inspect` feature) predates the colour API refactor and does not compile; it needs a rewrite against `to_rgb_linear_f32`/`to_rgb_linear_s44`.

## [0.8.1] - 2026-06-16

### Added

- `VsfType::uint(u64)` — constructs the narrowest unsigned VSF type that holds the value (EWE: `u3`/`u4`/`u5`/`u6` by magnitude), so a caller passes one `u64` and it's stored at minimal width. (`IntoVsfType for u64` stays fixed-width `u6`.)

## [0.3.2] - 2026-02-20

### Added
- **Scene graph primitives** - Full set of renderable object types: `rob` (rectangle), `roc` (circle), `ron` (container), `roe` (ellipse), `rol` (line), `rop` (path), `roo` (polyline), `ror` (NURBS), `rox` (spline), `rot` (text), `rou` (button), `roi` (image), `rof` (surface), `rom` (mask), `row` (group), `rog` (gradient), `rok` (stroke)
- **Dual-pipeline colour conversion** - Separate spectral and legacy pipelines with Rec2020 support; new `RgbLinearF32`, `RgbLinearF64`, `XyzF32`, `XyzF64` types
- **Theming system (`themes.rs`)** - Centralized colour scheme with five built-in themes: `dark`, `light`, `solarized-dark`, `nord`, `gruvbox-dark`; all inspect output colours routed thru `Theme` struct
- **`inspect_vsf()` function** - Public API for programmatic VSF inspection
- **Spirix tensor/colour integration** - `VsfType::tensor` and colour types display via spirix feature

### Fixed
- **Colour API renames** - `RgbLinear` → `RgbLinearF32`, `Xyz` → `XyzF32`, `Lab` → `LabF32`, `Lch` → `LchF32`, `Oklab` → `OklabF32`, `Oklch` → `OklchF32` (F64 variants added thruout)
- **Method renames for consistency** - `to_rgb_linear()` → `to_rgb_linear_f32()`, `from_rgb_linear()` → `from_rgb_linear_f32()`, etc.
- **Toka Tree reorganization** - Consolidated `toka_tree.rs` decoder into `types/toka_tree.rs`; removed separate decode file

### Changed
- **Colour inspect wiring** - All hardcoded RGB values in `inspect.rs` replaced with theme accessors (`col_ro()`, `col_colour()`, `col_size()`, `col_pass()`, `col_fail()`, `col_hint()`, `col_punct()`)
- **Spirix dependency** - Updated to `spirix = "0.0.7"` (no longer requires local path)
- **Opcode hints (vsfinfo)** - Inline hints showing opcode meaning in literal format (e.g., `{ps} # push`, `{fr} # fill rect`) *(moved from 0.3.1)*

## [0.3.0] - 2026-02-05

### Added
- **Opcode type (`VsfType::op`)** - Added dedicated type for executable bytecode with two-character ASCII identifiers
- **Literal VSF format (vsfinfo)** - `vsfinfo` now displays 1:1 file representation showing exact wire format with colour-coded syntax
- **Proper bracket notation (vsfinfo)** - Semantic distinction between `⦉⦊` (interpreted values) and `{}` (opcodes)
- **G^0* base notation (vsfinfo)** - Hexadecimal display with proper mathematical base prefix (replaces legacy 0x)

### Fixed
- **Eagle Time millisecond precision (vsfinfo)** - Now correctly calculates milliseconds from oscillation counts for integer types (eu6, ei6) instead of truncating to .000
- **Removed DEBUG prints (vsfinfo)** - Cleaned up debug output from Eagle Time metadata parsing

### Changed
- **BREAKING**: VSF format version bumped to 7 (z7 y7)
- **BREAKING**: Backward compatibility set to v7 (cannot read v6 files due to opcode type addition)
- **Type size markers** - All type size indicators now use `⦉⦊` brackets (e.g., `z3⦉6⦊`, `hp3⦉31⦊`)
- **Crypto hash formatting** - Hash type sizes now use `⦉⦊` brackets consistently with other types

### Documentation
- Added link to full documentation at https://holdmyoscilloscope.com/vsf/
- Updated README with v0.3.0 feature descriptions
- Moved capability tokens to v0.4.0 roadmap

## [0.2.3] - 2025-01-XX

### Previous releases
- Camera RAW support with bit-packed tensors
- Ed25519 signatures and verification
- Eagle Time temporal encoding
- Huffman text compression
- Spirix arithmetic integration

---

[0.3.2]: https://github.com/nickspiker/vsf/compare/v0.3.0...v0.3.2
[0.3.0]: https://github.com/nickspiker/vsf/compare/v0.2.3...v0.3.0
[0.2.3]: https://github.com/nickspiker/vsf/releases/tag/v0.2.3
