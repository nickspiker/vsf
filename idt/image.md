# VSF visual data — the image container

The one normative page for a VSF file that holds visual data: a still, a burst, a video with its audio, the layers that compose them and the provenance that vouches for them. It sits on [`colour.md`](../colour.md) (the colourspace every characterization lands in), [`README.md`](README.md) (the four IDT classes), [`tiers.md`](tiers.md) and [`output.md`](output.md). [`fields.md`](fields.md) is the running catalogue of fields and proposals; this page is what a writer must emit and a reader may rely on.

Status, 2026-10-09: **the still-image slice is implemented** in `vsf::visual` and written by opsin. Video spans, waves, marks, lineage and signatures are specified here as they are designed; they are marked *(design)* until the code lands. Nothing on this page is a wire-type change: the container is `vsf` 0.9.x throughout.

## The container

A visual document is an ordinary VSF document — header, table of contents, sections — with three conventions on top.

0. **Every entry carries a content hash.** The table-of-contents `h` slot holds `blake3(body)` (written by `VsfBuilder::content_hashes(true)`). A body can be checked on its own (`Document::content_hash_ok`) and, later, signed on its own.
1. **Edits append; nothing original moves.** A document grows by *generations* (`container::append`): the new section bodies in the self-describing far form, then a complete new header whose table of contents lists every live section by absolute offset, then a fixed 16-byte trailer (`">\xC3\x85R\0VSF\n"` ‖ u64-BE header offset) naming that header. A reader follows the trailer; a reader that predates generations reads the original header at offset 0 and sees the capture as it was, hashes intact. Each generation header carries `gen`, `start`, `prev` (the previous header's offset and BLAKE3 — the history chain), `tool` and `action`. Undo is truncation to the previous generation's end, byte for byte. A torn append (crash before the trailer) is invisible: the reader proves the last good header and reports the orphaned tail.
2. **Section names are addresses.** `kind.id[.sub]` with the dotted names the container already permits: `track.b0`, `span.b0.s0`, `plane.b0.s0.sensor`, `graph.g1`, `view.v1`. Ids start with a letter. A name re-added in a later generation supersedes the earlier one; a name listed in a generation's `supersede` is a tombstone. Earlier generations still list what they listed.

## A still

```
visual                 manifest: tracks, default_view, title, text
track.b0               kind beam, role capture
span.b0.s0             first 0, count 1, data → plane.b0.s0.sensor
plane.b0.s0.sensor     the sensor plane
capture                exposure, optics, when, where, body, lens, mode, baseline
characterization       the tiered camera → VSF RGB mapping, observer
graph.g0               the default program: plane → black_white → [decode] → idt → view ops
view.v0                the default output of graph.g0
foreign                XMP / ICC verbatim, profile name, calibration signature
```

A burst is the same with `count N` and N planes (or one `[n, h, w]` plane); video is the same with a rate declaration on the track and spans whose frames are inline. **A still is the one-frame case; nothing about it changes when a track grows.**

## Sections and fields

Types are the wire names (`u` unsigned EWE, `i` signed, `f5` f32, `a` ASCII text, `x` text, `e6` Eagle time, `w` WorldCoord, `t_*` tensor, `p` bit-packed, `v(c)` wrapped under code `c`, `hb`/`hp` BLAKE3). **closed** = a reader fails loud on an unknown value; **open** = a reader surfaces it. "absent ⇒" is what a reader does when the field is missing.

### `visual`
| field | type | notes |
|---|---|---|
| `tracks` | `a` nl-joined | track ids in presentation order |
| `default_view` | `a` | a `view.*` name; absent ⇒ the first view |
| `title`, `text` | `a` | free text (`text` is ImageDescription's home) |

### `track.<id>`
| field | type | notes |
|---|---|---|
| `kind` | `a` closed | `beam` \| `wave` \| `timecode` \| `annotation` \| `data` |
| `role` | `a` closed | `capture` \| `derived` \| `display` \| `preview` \| `calibration` |
| `title` | `a` | |
| `channels` | `u` | wave channels; beam plane count |
| `codec` | `a` open | declared payload codec; absent ⇒ bit-packed |

### `span.<track>.<seq>`
| field | type | notes |
|---|---|---|
| `track` | `a` | |
| `first` | `i` | tukutahi frame name of the first frame |
| `count` | `u` | ≥ 1 |
| `data` | repeated | one per frame, in order: `a` (a `plane.*` name), `p` (inline bit-packed), `v(c)` (inline under codec `c`), or an **empty** `v` = a declared hole |

A gap between two spans is a hole; so is an empty `v` inside one. Decoders reset at every hole.

### `plane.<track>.<seq>.<name>`
| field | type | notes |
|---|---|---|
| `role` | `a` closed | `sensor` \| `display` \| `preview` \| `depth` \| `ir` \| `uv` \| `confidence` \| `motion` \| `matte` \| `mask` \| `alpha` \| `dark` \| `flat` \| `bias` \| `derived`; absent ⇒ `sensor` |
| `width`, `height` | `u` | |
| `channel_count`, `channel_names` | `u`, `a` nl-joined | |
| `layout` | `a` closed | `mosaic` \| `planar` |
| `cfa` | `t_u3 [th, tw]` | mosaic only: channel index per cell; 2×2, 4×4, 6×6 all fit |
| `black`, `white` | `t_f5 [k]` | raw counts, per channel |
| `encoding` | `a` closed | VSF's own: `linear` \| `gamma2` \| `g2t`, also spelled `gamma2tanh` (VSF RGB with HDR headroom, `colour.md`), quantised ×2ⁿ, bucket bottom. A source's own, kept as written (v10): `srgb` \| `bt1886` \| `adobe_rgb` \| `prophoto` \| `pq` \| `hlg`, codes 0 to 2ⁿ − 1 with white at the top code ([`fields.md`](fields.md#untouched-sources-and-prior--implemented-2026-10-20)). **Absent ⇒ `gamma2`** (the integer-plane rule) — every writer here says which |
| `primaries` | `a` closed | `vsf` \| `srgb` \| `display_p3` \| `adobe_rgb` \| `prophoto` \| `rec2020`: the primaries and white the samples are in. **Absent ⇒ `vsf`** (VSF RGB, or camera counts a characterization maps). Any other value requires `layout = planar` and three channels, and the graph decodes it with `decode` (v10) |
| `prior` | `a` closed | an IDT class: what was done to the samples before the file and is not recorded in it. **Absent ⇒ nothing**; `technical` ⇒ mechanics only; `creative` ⇒ a look, which is every camera JPEG. A writer that cannot tell writes `creative` (v10, [README](README.md#what-came-before-the-file)) |
| `depth` | `u` | bits per sample, 1–128 |
| `curve_start`, `curve_step`, `curve_counts`, `curve_values` | `t_f5 [k]`, `t_f5 [k]`, `t_u5 [k]`, `t_f5` | spectral sensitivity per channel, when characterized (the `spectral_response` codec, on the plane) |
| `maker_crop` | `t_u5 [4]` | `[x, y, w, h]`: the maker's recommended crop — a suggestion, never applied |
| `orientation` | `u` | EXIF 274; absent ⇒ 1. A hint for the `orientation` node; the plane itself is sensor-order |
| `at` | `e6` | Eagle instant of the capture (stills) |
| `parent`, `parent_plane` | `hp`, `hb` | lineage: the parent document's `hp` and the parent plane's content hash |
| `program` | `a` | the `graph.*` whose root produced this plane, when derived |
| `samples` | `p` or `v(c)` | **last** in the section, so a lazy reader's header pass is short. Roles `sensor`/`dark`/`flat`/`bias`/`mask`: lossless only (`p`, or `v('z'\|'x'\|'r')` wrapping `p` bytes). Other roles may be `v('a')` or an application code |

A plane's table-of-contents content hash is **the plane hash**: the identity lineage and attestation name. Crop and recompress make a new plane with a new hash; the parent's is untouched.

### `capture`
| field | type | notes |
|---|---|---|
| `exposure_s`, `f_number`, `t_stop`, `gain`, `iso` | `f5` | per frame as `[n]` tensors when `count > 1` *(design)* |
| `focal_mm`, `focal_35mm`, `subject_m` | `f5` | |
| `at` | `e6` | DateTimeOriginal as an Eagle instant — only when the instant is known |
| `at_text` | `a` | the source's own date-time text, verbatim (EXIF has no zone; the text is what it said) |
| `location` | `w` | WorldCoord |
| `altitude_m` | `f5` | |
| `make`, `model`, `sensor_module`, `lens` | `a` | |
| `mode` | `a` open | `single` \| `burst` \| `bracket` \| `average` \| `slitscan` \| `video` |
| `baseline` | `f5` | DNG `BaselineExposure`: the file's declared opening gain in stops. A Technical scalar the reader applies; never folded into an operator's exposure |
| `stack` | `a` | the `graph.*` whose program is the stack recipe |
| `cal_applied` | `a` nl-joined | the dark/flat/bias planes applied |
| `shading` | `a` | the flat-field plane |

### `characterization`
Today's `colour_profile` fields, byte for byte (`spectral_image::profile_fields`): `target`, `count`, `matrices [n,3,3]` best first, `sources` (open), `classes` and `tiers` (closed — [README](README.md), [tiers.md](tiers.md)), `illuminants` (for choosing among entries only), `transfers` (vestigial: read when present, no longer written), and per entry the optional scale `sensitivities [n,3]`, `gain_refs [n]`, `units` (closed: `reflectance` \| `si_radiance`, checked against the class; absent ⇒ the matrix carries magnitude, as in v9), the verbatim `dng_colormatrix1/2` + `dng_illuminant1/2`, `patches_camera/reference`, `cal_target_type/serial/timestamp`. Plus:

| field | type | notes |
|---|---|---|
| `observer` | `a` open | the matching functions the matrices target: `cie1931`, `cvrl2006`, …; absent ⇒ unstated |

A reader also accepts the section under its old name `colour_profile`. A pasted IDT is the first entry with `source = verichrome_paste`, its XYZ→camera matrix in `dng_colormatrix1`'s slot so a copy out is exact, its target in `cal_*`, and its pairing string in `foreign.calibration_signature`.

### `graph.<id>` — the layer program
A DAG of sources and ops, serialised as a **Stack Notation** (postfix) program in canonical post-order from the root, operands bottom-first (fluor's `Under` convention); shared sub-results via `ref i`. The bytes are a *value*: the same program yields the same plane offline, forever, and is hashable into lineage. Editors and players are the state machines; their transitions are the generations.

| field | type | notes |
|---|---|---|
| `ops` | `a` nl-joined | op names, program order (open vocabulary; the reserved names below) |
| `classes` | `a` nl-joined | IDT class per op (closed) |
| `arity` | `t_u3 [m]` | operands popped — lets a reader keep the stack balanced through an op it does not know |
| `param_counts`, `params` | `t_u5 [m]`, `t_f5` | numeric params, sliced by count |
| `refs` | `a` nl-joined | per op a section-name operand (`plane.b0.s0.sensor`, `span.b0.s3`) or `-` |
| `root` | `u` | the op whose result is the output; absent ⇒ the last |
| `hash` | `hb` | BLAKE3 of the canonical program |

Reserved ops (the still's default graph uses the first three, `decode` when its plane is in foreign primaries, and the view ops): `plane` (0, ref) · `black_white` (1) · `decode` (1; the plane's primaries and encoding into VSF RGB, Technical) · `idt` (1, entry index) · `orientation` (1, EXIF code) · `crop` (1, x y w h) · `baseline` (1, stops) · `exposure` (1, stops) · `dr_curve` (1, coefficients) · `encode` (1, space transfer bits). *(design)*: `frame`, `frames`, `const`, `ref`, `demosaic`, `matrix`, `curve`, `contrast`, `white_balance`, `scale`, `mean`, `median`, `sum`, `max`, `min`, `sub`, `div`, `slitscan`, `under(mode)`, `alpha`, `mix` — bursts, composites and regional edits are all compositions of these. Technical ops precede Creative along every path; every op after `idt` in the default graph is arity 1, so a reader can strip the Creative ones and the program stays well-formed. An edit is a **new graph and view in a new generation**; the samples never change.

### `view.<id>`
| field | type | notes |
|---|---|---|
| `graph`, `root` | `a`, `u` | which program, which op |
| `space` | `a` | `vsf_rgb_linear` |
| `encoding` | `a` closed | `linear` \| `gamma2` \| `g2t` (or `gamma2tanh`); **absent ⇒ `gamma2`**, as for a plane: untagged is always VSF RGB, gamma 2 |
| `bits` | `u` | |
| `purpose` | `a` closed | `default` \| `preview` \| `print` \| `technical` |

### `foreign`
| field | type | notes |
|---|---|---|
| `xmp` | `v('b')` | the source's XMP packet, verbatim |
| `icc` | `v('b')` | the source's ICC profile, verbatim |
| `profile_name` | `a` | DNG `ProfileName` |
| `calibration_signature` | `a` | DNG `CameraCalibrationSignature`, or the one a paste wrote |

Foreign *metadata* is kept verbatim. Foreign *renderings* (a maker's JPEG preview) are not: a preview is a `plane` with `role = preview` and lineage, re-rendered from the sensor plane, which any reader may regenerate. *(planned 2026-10-17)*: a preview of content with light above diffuse white is `encoding = g2t` at 8 bits, so thumbnails keep their highlights on an HDR display; SDR content stays untagged gamma 2 (`colour.md`, "8-bit Pictures With Headroom").

### `provenance`
Unchanged from `spectral_image` (`handle`, `calibration_hash`, `camera_ihi`, `identity`). *(design)*: superseded by `attest`/`witness`/`run` from `lumis/docs/provenance.md`, `lineage`, `change` per generation, and `signature.<n>` manifest signatures — see [`fields.md`](fields.md).

## Absence

- No `characterization` and no `primaries` ⇒ the samples are VSF RGB (the rule of [README](README.md#absence-is-a-statement)).
- No `prior` ⇒ nothing was done to the samples before the file.
- No `graph` ⇒ render the primary plane through `black_white → idt(0)`.
- No `encoding` on a plane ⇒ `gamma2`. The legacy `spectral_image` section reads as **linear** — its only writer, opsin, wrote linear counts — and a legacy document reads as a one-track visual whose graph's `plane` op references `spectral_image`.
- No `role` on a plane ⇒ `sensor`; no `orientation` ⇒ 1; no `default_view` ⇒ the first view.

## What a reader may rely on
- Every live section is reachable from the latest table of contents by absolute offset; no scan.
- A plane's samples are the last field of its section, so `Document::packed` locates them from the fields before it and `packed_rows_u16` reads rows without unpacking the rest.
- Byte ranges below a generation's `start` were never written by that generation or any later one.
- Content hashes are over the body bytes at `o..o+b`, exactly as `vsfinfo verify` has always checked.
