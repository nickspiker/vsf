# Changelog

All notable changes to VSF will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Legacy conversions are chromatically adapted; Rec.2020 is built in LMS.** Every entry matrix into VSF RGB now folds in a von Kries adaptation (in Stockman & Sharpe 2000 10° LMS) from the source white to Illuminant E, so sRGB, Adobe RGB and Rec.2020 white land exactly on VSF `[1,1,1]` and back. Before, the matrices normalised the source to D65 and VSF to E and composed them unadapted: a D65 neutral came in off the R=G=B axis (sRGB white ≈ VSF `(1.20, 0.95, 0.91)` on the way out), which VSF's YCbCr then paid for in chroma. Rec.2020's wavelength primaries are now evaluated under the SS2000 10° fundamentals with D65 from the CIE 2019 SPD under the same fundamentals — CIE 1931 no longer touches the Rec.2020 path. `SRGB2VSF_RGB`, `ADOBE_RGB2VSF_RGB`, `REC2020_2VSF_RGB`, their inverses and S44 twins all change (max element shift ≈ 0.18); `VSF_RGB2XYZ`, `XYZ2VSF_RGB` and the LMS matrices are unchanged. Files converted from legacy spaces by earlier versions hold the unadapted values; re-convert from source to pick up the neutral white.
- **`colour.md` and the module docs now describe the conversion the code performs.** VSF RGB is the connection space (`Source → linear → VSF RGB → linear → Target`); wavelength-defined spaces enter via SS2000 10° LMS, xy-defined spaces via 1931 XYZ because an xy definition supports no other observer; adaptation is von Kries in LMS. The old text claimed everything went "thru LMS using CIE 2006 2°" and cited "accumulated transformation errors" — neither was true of the code, and the second isn't true of linear algebra.

- **`ProfileGrade` is `ProfileTier`; the `colour_profile` wire field `grades` is `tiers`.** The values (`unit`, `model`, `assumed`) are unchanged. "Grade" is this format's audience's word for the *creative* colour pass, so the characterization's trust axis was named with the industry's word for Creative — the exact opposite of what it means. "Tiered characterization" was already the docs' phrase. No file was ever written with `grades`, so there is no compatibility read: the wire name simply changed. The Rust rename is an API break for anyone matching on the enum or naming the field.

### Added

- `XYZ_D50_2VSF_RGB` (+ S44): `XYZ2VSF_RGB` with a D50 → E adaptation folded in, for ICC colorant tags, which the ICC spec adapts to the D50 profile connection space. `vsfimg` uses it for profiled input; untagged input still takes `SRGB2VSF_RGB`.
- `tools/generate_constants.rs` emits the `spirix`-gated S44 constant blocks and the committed literal style itself, so regenerating no longer clobbers hand-added blocks. The two 1nm observer tables are `#[rustfmt::skip]`, one value per line.
- `idt/` — the Input Device Transform specification: one page per class (Absolute, Relative, Creative, Technical), the tier axis, and the full image field set with the defaults absence implies and the proposed additions. `colour.md` and `README.md` link it.

## [0.9.1] - 2026-07-04

### Fixed

- **Format version markers now say v9.** 0.9.0 shipped the spirix 0.1 wire break (see below) but still wrote `z8`/`y8`, so old readers would accept new spirix-carrying files and silently misdecode them. 0.9.0 is yanked; files written by it should be re-encoded. `VSF_VERSION = 9`, `VSF_BACKWARD_COMPAT = 9`.

### Added

- **Compile-time version gate.** A const assertion ties `VSF_VERSION` to the semver-breaking number in `Cargo.toml` (minor while 0.x, major at 1.0+). A version bump without the matching format-version bump no longer builds, and since `cargo publish` runs a verify build, it can no longer be published either. A second assertion enforces `VSF_BACKWARD_COMPAT <= VSF_VERSION`.

## [0.9.0] - 2026-07-04 (YANKED — z/y markers were left at 8 despite the spirix wire break)

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
