# Changelog

All notable changes to VSF will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.12.0] - format v12 (reads v9, v10 and v11 unchanged)

### Added

- **A plane can keep a source's samples exactly as written** ([`idt/fields.md`](idt/fields.md), "Untouched sources and prior"; the reasoning in [`idt/README.md`](idt/README.md), "Encodings are not transforms"). A plane gains closed `primaries` (`vsf`, absent; `srgb`, `display_p3`, `adobe_rgb`, `prophoto`, `rec2020`), and `encoding` gains a source's own transfers (`srgb`, `bt1886`, `adobe_rgb`, `prophoto`, `pq`, `hlg`), whose codes run 0 to 2ⁿ − 1 with white at the top code. The graph's new Technical `decode` op takes such a plane into VSF RGB; `Visual::still` puts it after `black_white`. `Plane::encoded` builds one, `Encoding::light` / `light_f32` decode any encoding's code or signal, `Primaries::to_vsf_rgb` gives the matrix, and `Plane::vsf_linear_samples` does the decode once for an in-memory model (`to_spectral_image` uses it). Foreign primaries require a planar plane of three channels, checked on write and on read. `tests/foreign_planes.rs`.
- **`prior` on a plane**: what was done to the samples before the file existed and is not recorded in it, as an IDT class. Absent ⇒ nothing; `technical` ⇒ mechanics only; `creative` ⇒ a look, which is every camera JPEG. A writer that cannot tell writes `creative`, and `Plane::encoded` starts there. The device stays in `capture` and the event with its evidence belongs to provenance.
- Display P3 and ProPhoto RGB matrices (`DISPLAY_P3_2VSF_RGB`, `PROPHOTO2VSF_RGB` and their inverses, f32 and S44), generated like sRGB's and Adobe RGB's: thru 1931 XYZ, the encoding's white carried to Illuminant E by von Kries scaling in SS2000 10° LMS. The generator takes each space's white point (ProPhoto's is D50), its doc template escapes `[1,1,1]` and `[red, green, blue]` (rustdoc read them as links), and its spectral template matches the committed comments; the existing matrices regenerate unchanged.
- `colour::transfer::{bt1886_light, prophoto_light}`: the BT.1886 reference display (a 2.4 power) and ROMM RGB's 1.8 power with its linear toe.
- `colour::display::{abl_peak, abl_headroom}`: an emissive panel's brightness limiter. From a panel's measured white windows (area → peak), the peak a picture can have is the peak of the window that draws the same mean light, and `abl_headroom` finds the headroom the panel can sustain for a given picture, for `display_light` to aim at ([`idt/output.md`](idt/output.md), "Peak brightness depends on how much is lit").

- `colour::transfer::{pq_light, hlg_light}`: BT.2100 PQ and HLG signals → light relative to diffuse white (BT.2408: 203 cd/m² for PQ, 75 % of the HLG signal), light above white kept. One copy for every app that ingests HDR (photon previews, opsin's JPEG XL).
- `colour::display`: the display mapping, separate from storage ([`colour.md`](colour.md), "Display Mapping"). `display_light(l, headroom, content_peak)` is linear when the content fits the display and otherwise linear to a knee at 0.75 of peak with an extended-Reinhard shoulder that reaches peak at the content's brightest value; `display_lut_u8` builds the per-image table; `g2t_peak(bits)`.
- The flat `image` section carries `encoding`: `builders::compressed_image_encoded` writes it (nothing for `gamma2`), `parse_compressed_image` returns it as `colour_encoding`, and `vsf::image::decode_for_display(data, headroom)` maps a `g2t` image for the display (`decode` is headroom 1). `DecodedImage` gains `encoding`. `tests/flat_image_encoding.rs`.
- `g2t`, the short spelling of the `gamma2tanh` encoding: a reader accepts either, and writers here write `g2t`. No version change (no v11 writer had been deployed). Closed vocabularies may now list aliases (`closed_vocab!`, `"long" | "short"`).

### Changed

- `Encoding` gains six variants and `Plane` gains `primaries` and `prior` (breaking for exhaustive matches and struct literals outside this crate). `Graph::default_still` takes the plane's `primaries`.
- `colour::display::scene_light_u8` decodes thru `Encoding::light`, unchanged for VSF's own encodings.
- The flat `image` section refuses a source encoding on write and read: it has no primaries, so it is always VSF RGB.

### Fixed

- `colour::transfer` imported `num_traits::Float` without the `not(feature = "std")` gate the sibling modules use, an unused-import warning in every std build.
- `Visual::to_spectral_image` ignored the plane's `encoding`, so a `gamma2` or `g2t` plane reached the in-memory model as if its codes were linear counts. New `Plane::linear_samples` decodes an encoded plane (×2ⁿ, bucket bottom) to 16-bit linear counts with white at `65535 / max(1, peak)`, so a `g2t` plane's highlights sit above white; the bridge uses it (and limbus's raw reader). Linear planes are untouched.
- The inspector showed a `v`'s size with a hard-coded width marker `3` and the byte count minus one, matching neither the wire nor the docs. It now shows what the wire carries: the length in bits at the width the encoder wrote (100 bytes → `vb4⦉800⦊`). The `VsfType::v` docs for `b` name the bits convention; a test holds the inspector to the encoder's own bytes.
- A `view` with no `encoding` reads as `gamma2`, the same as a plane: untagged is always VSF RGB, gamma 2. It read as `linear` before. Every writer in this crate emits `encoding`, so no file written here changes meaning ([`idt/image.md`](idt/image.md)).

### Format

- `z` 12, floor 9. A plane may carry `primaries` and `prior`, the closed `encoding` vocabulary gains six values, and the graph gains `decode`. No byte of an existing type changes and every v11 file reads unchanged. The bump stops a v11 reader from ignoring `primaries` and reading an sRGB plane as VSF RGB; a v11 reader refuses the new encodings by name.

### Docs

- `idt/README.md`: "Encodings are not transforms" (capture, encoding and medium told apart by what sits at each end; an encoding has no class and no tier; output-referred encodings bundle a reference medium) and "What came before the file" (`prior`, and where colour stops and provenance starts). The tier example no longer calls a JPEG read as sRGB an Absolute characterization.
- `idt/output.md`, "Peak brightness depends on how much is lit": window curves, the equal-power rule, the sustained case, and what the model leaves out. `idt/fields.md` proposes `peak_area`, `peak_luminance` and `peak_sustained` on `medium`.
- `idt/image.md`, `idt/layers.md` (the import rule keeps a named encoding untouched), `colour.md` (ProPhoto's D50 white; the adaptation keeps an encoding's definition; brightness-limited headroom).
- `colour.md`, "8-bit Pictures With Headroom" *(planned)*: 8-bit pictures and previews use `gamma2tanh` only when the content has light above diffuse white; SDR stays untagged gamma 2; the flat `image` section gains `encoding` (absent ⇒ `gamma2`); AV1 previews mark their transfer unspecified.
- `idt/README.md`, "What a spectral profile is, and why": what a profile that keeps the camera's per-channel response buys over a fitted matrix (any light, any observer, any channel count, an error bar, an upgrade to Absolute), what it cannot do, and that DNG has no tag for it. "Absolute and Relative are ICC's intents, spectrally": the same distinction as ICC's colorimetric intents, with two differences: the illuminant is divided out per wavelength rather than thru a chromatic adaptation model, and the reference is a perfect reflector under the light, not the medium's own white.
- `idt/README.md`, "Why not ACES" rewritten as "Relation to ACES", facts only: the observer fixed in the encoding (1931 xy primaries and white), the spectral sensitivities used to build an IDT and then dropped, an IDT that states no class or tier (looks kept apart by the LMT convention, not by a field), and the 1.x per-channel tone scale with its hue shifts and blue-light artefacts, which ACES 2.0's output transform replaced. Credits ACES for its scene-referred interchange and for building IDTs from spectral sensitivities.

## [0.11.0] - format v11 (reads v9 and v10 unchanged)

### Added

- **`gamma2tanh`, VSF RGB with HDR headroom** ([`colour.md`](colour.md), "HDR: Gamma 2, Then tanh"): `encoded = tanh(√linear)`, per channel, decoded as `atanh(encoded)²`. Diffuse white (linear 1.0) lands on code 194 of 256; the top code is 3.28 stops above it at 8 bits, 3.86 at 10, 4.34 at 12 and 5.12 at 16, and shadows are plain gamma 2. Transfer functions in `colour::convert` for f32 (u8, u16, any depth from 1 to 16 bits, RGB triples) and S44, `ColourValue::{to,from}_linear_gamma2tanh`, and `gamma2tanh` as a plane/view `encoding` ([`idt/image.md`](idt/image.md)). Encoders settle the tanh estimate against the decoder, so decode → encode is exact at every code (S44 at 16 bits: within one code). `tests/gamma2tanh.rs`.
- `codec::model_of`: a `vc` value's model (raster, pyramid with its root count, series) from its header alone, so a reader can leave a pyramid coded for its own decoder.

### Fixed

- **Gamma 2 S44 at 16 bits now round-trips at every code.** S44's square root of a decoded code could land a hair under it, and truncation took it one low: every odd code from 257 up (64,256 of 65,536) came back one less. The S44 encoders now settle their estimate against the decoder, which S44 can do exactly here because it decodes every 16-bit code strictly above the one below. f32 and 8-bit S44 were already exact and are unchanged. `tests/gamma2_round_trip.rs`.

### Changed

- `ColourValue` gains two required methods, `to_linear_gamma2tanh` and `from_linear_gamma2tanh` (breaking for implementors outside this crate).

### Format

- The closed `encoding` vocabulary gains `gamma2tanh`. A 0.10 reader refuses a plane that uses it by name; every other byte reads as before.
- `z` 11, floor 9: v11 changed no byte of an existing type, so v9 and v10 files read unchanged.

## [0.10.0] - format v10 (reads v9 unchanged)

### Changed

- **What is built is what is read.** A 1-D tensor reads back as the `Tensor` it was built as, shape `[n]`; the `Vector` type and the `v_*` variants are gone (the compact `[t][n][count][type][size]` wire form is unchanged, and every contiguous tensor now writes its header through one canonical path: the u0, f5, f6, j5 and j6 tensors used to write a 1-D tensor in the long form). Until v9 a reader matching `t_*` found nothing in a 1-D field and said nothing. Migrate: match `t_*` only. `tests/round_trip.rs` pins every tensor type, bit-packed width and scalar.

### Added

- **Context-coded tensors, `vc`** (`vsf::codec`): integer tensors range-coded under adaptive context models (raster, quadtree pyramid with an optional reference stream, series), wrapped as `v` data with encoding byte `c`; `codec::decode` gives back the tensor exactly. Beat whole-file zstd by a fifth on mahere's map cells. `tests/codec.rs`.

### Format

- `z` 10, floor 9: v10 changed no byte of an existing type, so v9 files read unchanged and `vc` is skipped by a v9 reader as wrapped data.

## [0.9.x]

### Added

- **The visual container, still slice, and append-only generations (M1 of the VSF visual-data plan, 2026-10-09).** `vsf::visual` is the one model for stills, bursts and (later) video — specified in [`idt/image.md`](idt/image.md): `visual` manifest, `track.<id>`, `span.<track>.<seq>`, `plane.<track>.<seq>.<name>` (role, geometry, CFA, black/white, **`encoding` always written**, `maker_crop`, `orientation`, lineage), `capture` (the EXIF triangle, optics, Eagle instant, `WorldCoord` location, body/lens, `baseline`), `characterization` (today's `colour_profile` fields plus `observer`), `graph.<id>` (the layer program as a canonical Stack Notation DAG — `plane → black_white → idt → view ops`, an op's IDT class carried per op; the `idt` op takes the class of the entry it indexes), `view.<id>`, `foreign` (XMP/ICC verbatim, profile name, calibration signature) and `provenance`. `Visual::write` emits generation 0 with a content hash on every entry and the plane's samples last, so a lazy reader's header pass is short; `Visual::read` / `read_document` / `read_file` read new documents, hybrid appended ones, and legacy five-section `spectral_image` files as a one-track visual whose plane is **linear** (its only writer wrote linear counts — no migration table needed); `to_spectral_image` / `from_spectral_image` bridge the in-memory model both ways. `container::append`: `Appender` builds a generation — far-form bodies, a header listing every live section by absolute offset with inline `gen`/`start`/`prev`(offset + BLAKE3 of the previous header)/`tool`/`action`, its own `hp` over bodies ‖ header, and a 16-byte trailer naming the header — `append_to_file` writes it with fsyncs in the order that keeps every intermediate state readable, and `truncate_file` is undo. `Document::open` follows the trailer, falls back to the **last complete generation** after a torn append (walking forward from the original's end for a header whose `start`, `l` and trailer all agree) and reports the orphaned tail; `generations()` walks and verifies the chain; `generation_hash_ok` recomputes a generation's `hp`; `checkout(n)` is the first *n*-th-generation bytes, byte for byte; `undo_len` is where to truncate. `Visual::still_edit_generation` appends a new graph + view for an edit without touching a sample. `vsfinfo tree` shows the generations and the visual model (tracks, spans, planes, capture, characterization, graph disassembly, views, foreign); `verify` checks every generation's hash and the chain and skips inline header fields; new `history`, `checkout N OUT` and `undo` subcommands. `VsfBuilder` fix: `l` (file length) now includes unboxed blobs — earlier builders wrote it short, and `Document` tolerates those files. Tests: `visual_still.rs`, `container_append.rs` (including a torn third generation falling back to the second), `container_lazy.rs`. Version stays 9: every change is additive and a v9 reader sees generation 0 as the document it always was.
- **Lazy, partial reads — the first piece of the visual container (M0 of the VSF visual-data plan, 2026-10-09).** `container::Document` opens any `ByteSource` (a slice, a `Vec`, a `File` — positional reads, no shared cursor) by reading the header alone, lists the table of contents, and fetches a section's bytes only when asked; `Document::fields` walks a body recording where every value sits, with bit-packed planes (`p`) and wrapped payloads (`v`) located from their headers and never copied (`container::skip`); `Document::packed` / `packed_rows_u16` / `packed_range_u16` read just the rows a viewer needs. `types::BitPackedView` unpacks any sub-range, row band, tile or slab (one plane of a `[k,h,w]` tensor, which may start mid-byte) straight from the bytes — zero-copy over memory, a byte window over a file — at every depth 1–128, with `byte_window` telling a caller which bytes a range touches; `types::BitPacker` packs samples in any batches to bytes identical to `pack_*`. Both are pinned against the whole-tensor paths in `tests/bitpacked_slicing.rs` (every depth 1–16 × awkward widths × every start). `VsfSection::encode_far` emits the self-describing far form at any offset (what an appended body will use); `encode_at_offset` is unchanged in output. `VsfBuilder::content_hashes(true)` / `SectionMeta::content_hash` put `blake3(body)` into each table-of-contents entry (`hb`, the slot `vsfinfo verify` has always checked), and `Document::content_hash_ok` checks one entry without re-hashing the file — the hook append-surviving manifest signatures build on. Off by default: existing writers' bytes are unchanged.

### Fixed

- **Hostile sizes now fail loud instead of wrapping.** `parse_bitpacked_tensor` computed `elements × bits` unchecked (a forged `[2^40, 2^40]` shape wrapped to a small allocation and a read past the end); `parse_wrapped` did `(bits + 7) >> 3` and an unchecked pointer add (a length near `usize::MAX` wrapped — found by `tests/overflow.rs` on its first run); the `v` encoder's `len << 3` wrapped silently past 512 MiB on 32-bit targets (now a message naming the cap). `Tensor::try_new` is the total constructor for reader-facing code.
- README: bit depths are 1–128 (not 1–256); the "zero-copy mmap" claim now describes what exists (`Document`/`BitPackedView`); per-section signing describes what `sign_section` actually signs (the whole document) and what content hashes add.

### Changed

- **`idt/`: the four classes run both ways, and a tier says who measured — not whether they were honest.** New [`idt/output.md`](idt/output.md): a medium (emissive / reflective / transmissive) is the IDT in reverse, and the same four classes describe it; its white and black are *measurements* on the kind's scale, per primary, carrying a tier — a brightened paper reads above 1 and is kept so, never normalised, with the illuminant and geometry that make the number mean something; which class each kind can physically keep (an OLED can be Absolute, paper is the Relative case made of paper, e-ink's gamut makes Creative unavoidable); and what a renderer does about a medium's range is an op with a class — one neutral scale onto media white is Technical, a black-point lift, a gamut squeeze or an adaptation to media white is Creative, which is the distinction ICC's "relative colorimetric" bundles away. `tiers.md`: the Sigma fp writes a *per-colour-mode* look into its DNG Model slot (measured at 0.57×/0.80× true chroma against 0.92× for the Adobe-derived matrix and a 2.5 ΔE floor, five VERICHROME-target frames, 2026-10-08), so the tier cannot be read as honesty; `source` says whose, and a reader ranks sources within a tier. `fields.md`: proposed `medium` section and `maker_crop`; the gamma-direction note resolved; the open DNG-carriage question recorded. `IdtClass` and `ProfileTier` doc comments carry the sharpened definitions (Technical = no claim and no taste: mechanics and instrument remaps). No wire change.

### Fixed

- **`WorldCoord` was broken twice over; every stored value predating this entry is suspect.** The bit-field packing (`face<<59 | u<<30 | v`) stuffed `BASE = 960,383,883` (a 30-bit quantity) into a 29-bit u field, so every point with v0-weight > 0.559 — ~19% of each face, the region around v0 — overflowed into the face bits and truncated u: decodes teleported to other faces. Independently, six `FACES` entries paired vertices at distance 2.0 (not icosahedron-adjacent) — the table described a solid that does not exist. Restored the reference arithmetic divmod packing `(x·BASE + y)·20 + face` with the fold (bit fields can never hold it: 5+30+30 = 65) and verified tables. Also fixed a boundary bug latent in the reference codec itself: points exactly on a face edge (u+v = 1.0 — the antimeridian hits this on grid-aligned input) folded inside diagonal grid cells, whose mirrored codes collide with unfolded diagonal codes, decoding reflected across the hypotenuse by up to ~6,600 km. Encode now clamps overshoot onto the hypotenuse and never folds diagonal cells. The type had zero tests; it now has a suite (face-table geometry, global grid < 1 cm by chord metric, near-vertex sweeps over the old overflow region, re-encode stability, face coverage, words round-trip).

### Changed

- **The dymaxion orientation is pole-vertex, Greenwich-aligned — raw `w` values change meaning.** Vertex 0 is the north pole, vertices 1–5 the upper ring at lat atan(1/2) ≈ 26.565° (lons 0°, 72°, …), 6–10 the lower ring (lons 36°, 108°, …), vertex 11 the south pole; unit vertices, tables shared verbatim with mahere-coord and the icosahedron reference repo. Chosen on symmetry grounds: the polar axis is a 5-fold axis, so lat/lon's rotational symmetry shares its largest cyclic subgroup with the solid (C5 > C3 > C2), and the coordinate system's polar degeneracy lands exactly on the grid's two degree-5 points. Land-weighted distortion was measured orientation-insensitive (~1%), so the symmetry argument decided. Combined with the packing fix above, any `w` raw value or wire bytes written by earlier versions decode to a different place on Earth; nothing is known to have persisted them.
- **`flatten_at`/`from_wire` widths below `'6'` are spatial re-encodes, not bit truncations.** The divmod packing's high bits approximate only the barycentric x axis, so the old truncation named a strip across a face, not a region — the docs' "~10 km at w5" claim was never true. Each coarse width now re-encodes at base floor(√(2^bits/20)) (w3 ≈ 2,350 km cells, w4 ≈ 124 km, w5 ≈ 481 m) and `from_wire` reconstructs the coarse cell's center. Widths are not bit-nested; conversion goes through the sphere.

### Added

- **`WorldCell` — the Morton-coded dymaxion type, wire tag `wm` (w-family lowercase member).** Layout `[4 bits diamond 0–9][60 bits Morton(u,v)]` over the 10-diamond pairing of the shared tables, ~6.6 mm at 64 bits. The hierarchical complement of `WorldCoord`'s density: truncating 2k low bits yields the enclosing cell 2^k coarser, a tile address is a bit prefix, containment is a mask-and-compare, and wire widths `wm3`–`wm7` nest by truncation (coarse decodes re-center so degraded points read as cell centers, not corners). `encode128`/`decode128` extend the layout to `[4][124]`, and the top 64 bits of the 128-bit encoding are exactly the 64-bit value — power-of-two quantization commutes with truncation (tested). f64 inputs bound meaningful depth at ~52; the layout is exact to 62. Codec is bit-identical to mahere-coord's.
- `WorldCoord::coarse32()`/`from_coarse32()`: the 32-bit coarse dense encode at `BASE32 = floor(√(2^32/20)) = 14,654` (~481 m steps, ~140 m mean error) — the second member of the dense family (64-bit fine, 32-bit coarse), independently encoded, not nested, by design.

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
