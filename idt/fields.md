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
| `entries[i].grade` | `grades` | `unit` \| `model` \| `assumed` | closed — see [grades.md](grades.md) |
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

0. **No `colour_profile`** — the samples **are VSF RGB**. Not "uncharacterized": a reader renders thru the identity camera matrix with Illuminant E as the white, which XYZ→VSF RGB maps to exactly (1, 1, 1). There is nothing to grade because nothing is claimed beyond what the format guarantees. *Implemented in opsin, 2026-09-22.* A RAW with no matrix is **not** this case — sensor counts are not VSF RGB — and it carries the identity at `assumed`, source `no_colormatrix`.
1. **No `view_transform`** — no ops. Display as characterized.
2. **No `curve`** — the entry's matrix is the characterization, bounded to its fit set. See [grades.md](grades.md#what-a-grade-is-not).
3. **Sample encoding** — integer planes are **gamma 2**, encoded with the square root and decoded with the square, per [`colour.md`](../colour.md) and the reference implementation. Float planes are linear. (There are no float planes yet; `samples` is integer-only.)

> **Note on direction.** The reference implementation in `src/colour/convert.rs` encodes with `sqrt` and decodes with the square — `let encoded = linear.sqrt()` — and lumis and opsin do the same. The prose in `colour.md` currently states the reverse (`encoded = linear*linear`). The implementation is authoritative; the prose is a known discrepancy as of 2026-09-22.

## `transfer` is vestigial

`ProfileEntry::transfer` was designed in with the colour_profile section (vsf 0.9.2) so a display-referred ingest could store a JPEG's sRGB-encoded values as-is and let the *reader* linearize. No writer took that path: opsin linearizes at ingest and stores linear planes, so every writer emits `linear` and nothing anywhere reads it back — the only reader is vsf's own parser, round-tripping it. Its own doc calls it "the transfer the *samples* carry," which is a property of the plane, stored on the entry. That misplacement is the reason identity entries kept being invented: they were the only place to say "this plane is linear." It is superseded by `encoding` below.

## Proposed additions — 2026-09-22

Status: **proposed** unless marked. Field names are the ones the class pages use.

| where | field | what | status |
|---|---|---|---|
| `spectral_image` | `encoding` | `linear` \| `gamma2` — the sample encoding, on the plane, defaulting per the absence rule | proposed; replaces `transfer` |
| entry | `sensitivity[k]` | sensor-plane exposure at white level, per channel. Absolute: J·m⁻² monochromatic-equivalent at the curve's peak. Relative: fraction of a perfect diffuse reflector under the solved illuminant | proposed. The scalar the entry is incomplete without |
| entry | `gain_ref` | the gain `sensitivity` was measured at | proposed |
| entry | `unit` | what 1.0 means in the output: `reflectance` for Relative; radiance in [`units.md`](../units.md) base units, SI interim, for Absolute | proposed |
| entry | `matrix` | **normalised** — the scale factored out into `sensitivity` | proposed change of meaning |
| entry | `illuminant` | Relative: which illuminant this entry was solved under, for **selection**. Absolute: absent — no illuminant in the transform | proposed change of meaning |
| entry | `transfer` | retired | proposed |
| `cal` | `emitter` | which source an Absolute scan was shot under, and its measured absolute output — provenance, like the target serial | proposed |
| `capture` (new section) | `exposure_s`, `t_stop`, `gain`, `shading` | the per-frame triangle, and an optional per-lens flat-field. Needed to go from sensor-plane exposure to scene radiance. Today only in EXIF headers | proposed — the schema system's own README already sketches a `camera` section with `iso`/`aperture`/`timestamp` |
| DNG bridge | — | `BaselineExposure` applied to the render, derived fresh, never into the slider | **implemented in opsin, 2026-09-22** — DNG-side only; a VSF has no field to carry it until `sensitivity` lands |
| class doc | `technical` | "makes no claim about how the scene looked and expresses no taste — value-preserving mechanics, and instrument remaps" | proposed, see [technical.md](technical.md#a-sharpening-of-the-definition) |
| opsin | `exposure` op | class `creative`, not `technical` — it is a second scalar | proposed, see [creative.md](creative.md#status-of-opsins-exposure-op) |
| `view_transform` | ordering | Technical ops precede Creative ops | proposed, see [creative.md](creative.md#the-layer-stack) |

Two things are decided by the class pages and not yet by fields: what 1.0 means for Absolute radiance (SI interim, base units when [`units.md`](../units.md) carries symbols), and where `shading` lives (own section, or a sidecar reference).

## Migration

About ten 16-bit VSF images exist as of 2026-09-22, all written by opsin with linear planes. Under the absence rule their integer samples would read as gamma 2. They need either re-encoding or an explicit `encoding = linear` once that field exists — which is the reason `encoding` is a field and not only a default.
