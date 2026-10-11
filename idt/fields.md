# Fields

Every field a VSF image carries, by section, with the name it has on the wire. Then the defaults that absence implies, and the additions proposed on 2026-09-22 with their status. The Rust names are in `src/spectral_image.rs`; the wire names are what `write` emits and `read` looks for.

## `spectral_image` — the samples and what they are

| Rust | wire | type | notes |
|---|---|---|---|
| `width`, `height` | `width`, `height` | unsigned | |
| `channels[k].name` | `channel_names` | string, newline-joined | k names |
| `channels[k].curve` | `curve_start`, `curve_step`, `curve_counts`, `curve_values` | f32 tensors | relative spectral sensitivity on a self-describing grid; written only when present. **Wins over any matrix when present.** |
| `layout` | `layout` | `mosaic` \| `planar` | closed vocabulary |
| `layout: Mosaic { cfa }` | `cfa` | u8 tensor `[tile_h, tile_w]` | channel index per cell; Bayer RGGB is `[[0,1],[1,2]]` |
| `samples` | `samples` | bit-packed integer tensor | mosaic `[h, w]`, planar `[k, h, w]`. **Integer only**, 1–128 bits |
| `black[k]`, `white[k]` | `black`, `white` | f32 tensors | per channel, in raw counts |
| `make`, `model` | `make`, `model` | string | written when non-empty. opsin also parks a decoder's error and the headerless guesser's verdict in `model` |

## `provenance` — the ihi identity

| Rust | wire | notes |
|---|---|---|
| `handle` | `handle` | `""` means omitted |
| `calibration_hash` | `calibration_hash` | 32 bytes |
| `camera_ihi` | `camera_ihi` | 32 bytes |
| `identity` | `identity` | 32 bytes; composed by callers that link ihi, vsf itself does not |

## `colour_profile` — the characterization

`target` is the space every entry maps to. v0's only legal value is `vsf_rgb`; a reader seeing any other target treats the file as uncharacterized.

| Rust | wire | type | notes |
|---|---|---|---|
| `target` | `target` | string | closed: `vsf_rgb` |
| `entries.len()` | `count` | unsigned | n |
| `entries[i].matrix` | `matrices` | f32 tensor `[n, 3, 3]` | camera → target. **Best first** |
| `entries[i].source` | `sources` | string, newline-joined | **open** vocabulary: `magic9`, `dng_colormatrix1`, `assumed_srgb`, `no_colormatrix`, … unknown still names a working matrix |
| `entries[i].class` | `classes` | `absolute` \| `relative` \| `creative` \| `technical` | closed — see [README](README.md#the-four-classes) |
| `entries[i].tier` | `tiers` | `unit` \| `model` \| `assumed` | closed — see [tiers.md](tiers.md). |
| `entries[i].illuminant` | `illuminants` | u16 tensor `[n]` | EXIF LightSource code, 0 = unknown. Today: display exposure scalar only, never adaptation |
| `entries[i].transfer` | `transfers` | `linear` \| `srgb` \| `gamma2` \| `gamma22` | **vestigial** — see below |
| `dng_colormatrix[0..2]` | `dng_colormatrix1`, `dng_colormatrix2` + `dng_illuminant1`, `dng_illuminant2` | f32 `[3,3]` + unsigned | verbatim XYZ→camera and its illuminant code, untouched — the question the entries answer |
| `patches` | `patches_camera`, `patches_reference` | f32 tensors `[p, 3]` | magic-9 solve inputs: camera-space patch means, reference values in VSF RGB |
| `cal` | `cal_target_type`, `cal_target_serial`, `cal_timestamp` | unsigned, unsigned, string | which target produced a `unit` entry |

## `view_transform` — the translateration log

Ordered ops applied after characterization, in `space`. The samples are never touched; this is interpretation a reader replays.

| Rust | wire | type | notes |
|---|---|---|---|
| `space` | `space` | string | v0: `vsf_rgb_linear` — after the elected matrix, before display encode |
| `ops[j].name` | `ops` | string, newline-joined | **open** vocabulary. v0 defines `exposure`; reserved `curve`, `contrast`, `skew_matrix`, `white_balance`, `dr_curve`. **A reader that meets an unknown op must surface it, never drop it** |
| `ops[j].class` | `classes` | closed, as above | |
| `ops[j].params` | `param_counts`, `params` | u tensor `[m]`, f32 tensor `[total]` | flattened; `param_counts` says where each op's slice ends |

opsin writes `orientation` (EXIF 274 code), `crop` (x, y, w, h), `exposure` (stops) and `dr_curve` (the rolloff's coefficients).

## What absence means

Absence is a specification, not a gap. Every field below has a defined reading when it is missing.

0. **No `colour_profile`** — the samples **are VSF RGB**. Not "uncharacterized": a reader renders thru the identity camera matrix with Illuminant E as the white, which XYZ→VSF RGB maps to exactly (1, 1, 1). There is nothing to tier because nothing is claimed beyond what the format guarantees. *Implemented in opsin, 2026-09-22.* A RAW with no matrix is **not** this case — sensor counts are not VSF RGB — and it carries the identity at `assumed`, source `no_colormatrix`.
1. **No `view_transform`** — no ops. Display as characterized.
2. **No `curve`** — the entry's matrix is the characterization, bounded to its fit set. See [tiers.md](tiers.md#what-a-tier-is-not).
3. **Sample encoding** — a plane without `encoding` is **gamma 2**, encoded with the square root and decoded with the square, per [`colour.md`](../colour.md) and the reference implementation. Float planes are linear. (There are no float planes yet; `samples` is integer-only.) *Since 2026-10-09 every writer says which:* a `plane.*` section in the visual container ([`image.md`](image.md)) always carries `encoding = linear | gamma2`, and the legacy `spectral_image` section reads as **linear** — its only writer, opsin, wrote linear counts — so the absence rule is reached only by a plane written by hand.

> **Note on direction.** The reference implementation in `src/colour/convert.rs` encodes with `sqrt` and decodes with the square — `let encoded = linear.sqrt()` — and lumis and opsin do the same. `colour.md` said the reverse until 2026-10-08; it now agrees with the code, and the code was always authoritative.

## `transfer` is vestigial

`ProfileEntry::transfer` was designed in with the colour_profile section (vsf 0.9.2) so a display-referred ingest could store a JPEG's sRGB-encoded values as-is and let the *reader* linearize. No writer took that path: opsin linearizes at ingest and stores linear planes, so every writer emits `linear` and nothing anywhere reads it back — the only reader is vsf's own parser, round-tripping it. Its own doc calls it "the transfer the *samples* carry," which is a property of the plane, stored on the entry. That misplacement is the reason identity entries kept being invented: they were the only place to say "this plane is linear." It is superseded by the plane's `encoding` (implemented 2026-10-09, [`image.md`](image.md)); the entry field is still written and read for the legacy section and ignored.

## Proposed additions — 2026-09-22

Status: **proposed** unless marked. Field names are the ones the class pages use.

| where | field | what | status |
|---|---|---|---|
| `plane.*` | `encoding` | `linear` \| `gamma2` — the sample encoding, on the plane, defaulting per the absence rule | **implemented 2026-10-09** in `vsf::visual` ([`image.md`](image.md)); replaces `transfer` |
| entry | `sensitivity[k]` | sensor-plane exposure at white level, per channel. Absolute: J·m⁻² monochromatic-equivalent at the curve's peak. Relative: fraction of a perfect diffuse reflector under the solved illuminant | proposed. The scalar the entry is incomplete without |
| entry | `gain_ref` | the gain `sensitivity` was measured at | proposed |
| entry | `unit` | what 1.0 means in the output: `reflectance` for Relative; radiance in [`units.md`](../units.md) base units, SI interim, for Absolute | proposed |
| entry | `matrix` | **normalised** — the scale factored out into `sensitivity` | proposed change of meaning |
| entry | `illuminant` | Relative: which illuminant this entry was solved under, for **selection**. Absolute: absent — no illuminant in the transform | proposed change of meaning |
| entry | `transfer` | retired | proposed |
| `cal` | `emitter` | which source an Absolute scan was shot under, and its measured absolute output — provenance, like the target serial | proposed |
| `capture` (new section) | `exposure_s`, `f_number`, `t_stop`, `gain`, `iso`, `focal_mm`, `at`, `location`, `make`, `model`, `lens`, `mode`, `baseline`, `shading`, … | the per-frame triangle, optics, when and where, body and lens, and an optional per-lens flat-field. Needed to go from sensor-plane exposure to scene radiance | **implemented 2026-10-09** ([`image.md`](image.md#capture)); opsin fills it from EXIF + GPS on every ingest. A zero rational (f/0, 0 mm from a manual lens) is carried as absent |
| DNG bridge | — | `BaselineExposure` applied to the render, derived fresh, never into the slider | **implemented in opsin, 2026-09-22**; carried as `capture.baseline` since 2026-10-09 |
| class doc | `technical` | "makes no claim about how the scene looked and expresses no taste — value-preserving mechanics, and instrument remaps" | proposed, see [technical.md](technical.md#a-sharpening-of-the-definition) |
| opsin | `exposure` op | class `creative`, not `technical` — it is a second scalar | proposed, see [creative.md](creative.md#status-of-opsins-exposure-op) |
| `view_transform` | ordering | Technical ops precede Creative ops | proposed, see [creative.md](creative.md#the-layer-stack) |

Two things are decided by the class pages and not yet by fields: what 1.0 means for Absolute radiance (SI interim, base units when [`units.md`](../units.md) carries symbols), and where `shading` lives (own section, or a sidecar reference).

## Proposed additions — 2026-10-08

Status: **proposed**. Fixed by [`output.md`](output.md) and the maker's-crop work in opsin; not yet fields.

| where | field | what | status |
|---|---|---|---|
| `medium` (new section) | `kind` | `emissive` \| `reflective` \| `transmissive` — closed; decides what 1.0 means and which classes the medium can keep | proposed |
| `medium` | `white[3]`, `black[3]` | the most and least the medium gives back, per primary in the output space, on the kind's scale: a perfect diffuser under the viewing illuminant (reflective), the open lamp (transmissive), a stated radiance (emissive). **Values above 1 are real and kept** — a brightened paper, a specular — never normalised | proposed |
| `medium` | `illuminant` | EXIF LightSource code the white and black were measured under — a brightener's white depends on the lamp's UV | proposed |
| `medium` | `geometry` | the measurement geometry (45/0, specular…) — gloss is a direction | proposed |
| `medium` | `tier` | `unit` (this sheet, this panel) \| `model` (the stock's datasheet) \| `assumed` (took 1 and 0) | proposed |
| `plane.*` | `maker_crop` | `[x, y, w, h]` in sensor pixels: the maker's recommended crop (DNG `DefaultCropOrigin`/`Size` offset by `ActiveArea`, or a body's published crop). A **suggestion**, never applied — the plane and the view show everything recorded, masked borders included; opsin's crop tool starts here. Technical by nature; distinct from the `crop` op, which is the operator's | **implemented 2026-10-09** on the plane ([`image.md`](image.md#planetrackseqname)) |

### DNG carriage — decided 2026-10-08

Implemented in opsin (`src/idt.rs`), Nick: "I'm down with your recommendation." A DNG cannot say what an IDT claims, but it can carry one so that every reader applies it, and it can carry a note beside it that says what it is.

0. **The matrix goes in `CameraCalibration1/2`**, the specification's own per-unit slot, which every DNG reader multiplies onto the ColorMatrix: `XYZ→camera = CameraCalibration × ColorMatrix`. The paste writes `CC_i = M × CM_i⁻¹` per slot, so the product is the IDT exactly at each slot's illuminant. The file's own `ColorMatrix1/2` is untouched — the factory matrix survives. `CameraCalibrationSignature` and `ProfileCalibrationSignature` are set equal (`VERICHROME <class> <tier> target#<serial> <scanned>`), so a reader using the file's embedded profile applies the calibration and one that swaps in its own profile sees no match, ignores it, and falls back to its own per-model matrix — never our correction on someone else's base.
1. **The claim goes in XMP** (tag 700), namespace `https://verichrome.cc/ns/idt/1.0/`, prefix `verichrome:` — `IdtClass`, `IdtTier`, `Observer` (the matching functions the solve targeted: a DNG assumes CIE 1931, chameleon solves for CVRL 2006), `Illuminant`, the nine exact rationals as `Matrix` (so copy out of a pasted file is verbatim, not `CC × CM` rounded), `TargetType`, `TargetSerial`, `TargetCalibrated`, `Scanned`, `Source`, `Signature`, `Pasted`. An existing packet is kept byte for byte and the description added beside it; a re-paste replaces its own description.
2. **No original byte moves.** The new tags need a bigger IFD0, so the paste appends a copy of IFD0 with the added entries at the end of the file and repoints the header's one 4-byte offset. `OriginalIFD0` and `OriginalLength` in the XMP make undo (`opsin --unpaste-idt`) a 4-byte write and a truncate — byte for byte the file it was, even after several pastes.

A reader without the XMP sees an ordinary calibrated DNG. opsin reads all three: it elects a `VERICHROME`-signed calibration as `unit` / `relative` (or whatever the XMP says — a factory matrix pasted between frames of one camera records itself as `model` / `absolute`), and applies `CameraCalibration × ColorMatrix` on every DNG regardless, as the specification requires.

## The spectral response — implemented 2026-10-10

From the chameleon `spectral.rs` study and Nick's direction: *encode the spectral response, ditch the chart, then relight the shot illuminant to the desired one.* The representation is specified in [`relative.md`](relative.md#the-representation--decided-2026-10-10); the fields live on `characterization`, beside the matrices they supersede (the matrices stay as the cache a reader without curves uses and a reader with curves checks against). Nothing here needs an absolute illuminant.

| where | field | what | status |
|---|---|---|---|
| `characterization` | `response_start`, `response_step` (`f5`), `response_count`, `response_channels` (`u`) | the linear band grid and the shape of the response: `channels` camera channels (three for a Bayer set, seven for a seven-channel camera) × `count` bands | **implemented** (`vsf::visual::SpectralResponse`) |
| `characterization` | `response_values` (`t_f5 [channels × count]`) | `P₀_c(λ) = S_c(λ)·E_ref(λ)`, channel-major, **one common scale** — relative channel heights are information | **implemented** |
| `characterization` | `response_prior` (`a`, open) | how the curves were regularised: `shortest` (path length; the default — a dye set's shape), `unimodal`, … A reader shows it; nothing depends on it | **implemented** |
| `characterization` | `response_light` (`hb`) | identity of the calibration light — BLAKE3 over the scan's target serial and 21 patch values. Two responses with the same light compare; a shot's illuminant ratio is relative to it; a measured `E_ref` for this light upgrades every response made against it to Absolute | **implemented** |
| `plane.*` | `curve_*` | unchanged: a channel's *measured* spectral sensitivity when one exists (monochromator, LED sweep). The response above is the chart-recovered effective curve under one light; the two are different claims and keep different homes | — |
| `graph` op `idt` | `observer`, `illuminant` params | the transform *derived* from the response at render time — `N × K` for an N-channel observer and a K-channel camera, fitted over a reflectance ensemble in a perceptual metric — the stored matrix read only when no response is present | proposed (M3) |
| `graph` op `relight` (new, Technical) | `ratio` | `ρ(λ) = E_shot/E_ref` on the response grid, with its source: a target scan under the shot's light (exact; the common case is footage from the scan's own light, `ρ = 1` as a fact), or a measured SPD over the reference. Never a single-patch scale, never the camera's white balance. Absent ⇒ 1 | proposed (M3) |
| `capture` | `spd` | a measured illuminant spectrum for the shot, verbatim, when one exists — what `relight` prefers under structured lights (LED, fluorescent), which a chart scan cannot fully resolve | proposed, optional |

What the study established (three Sigma fp frames + one Canon): the transform built from recovered curves over an ensemble matches the direct 21-patch fit on held-out patches within 0.1–0.4 % of white; a new frame's transform from three neutral patches alone landed within 0.7 % of white of its full-chart fit under broadband light (a study result, not a paste path); the two routes differ by 2–3 % of white on the chart and 3–4.5 % on spectra the chart lacks; quadratic smoothness priors produce two-humped curves no dye set has, the shortest-curve prior does not, and the data cannot tell them apart — so the prior is stated. Structured lights remain the limit of chart-free relighting and are why `spd` exists.

## Migration

About ten 16-bit VSF images exist as of 2026-09-22, all written by opsin with linear planes. Under the absence rule their integer samples would read as gamma 2. **Resolved 2026-10-09 without a table:** the legacy `spectral_image` section had one writer and it wrote linear, so `Visual::read` adapts a legacy document to a one-track visual whose plane is `linear` ([`image.md`](image.md#absence)). New files carry `encoding` explicitly — which is the reason it is a field and not only a default. The visual container itself is specified in [`image.md`](image.md); this page remains the catalogue of fields and proposals.
