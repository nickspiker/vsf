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
4. **No `primaries`** on a plane — VSF RGB, or camera counts that the characterization maps. *Since format v10.*
5. **No `prior`** on a plane — nothing was done to the samples before the file. A writer that cannot tell writes `creative` instead, because absence is the stronger claim. *Since format v10.*

> **Note on direction.** The reference implementation in `src/colour/convert.rs` encodes with `sqrt` and decodes with the square — `let encoded = linear.sqrt()` — and lumis and opsin do the same. `colour.md` said the reverse until 2026-10-08; it now agrees with the code, and the code was always authoritative.

## `transfer` is vestigial

`ProfileEntry::transfer` was designed in with the colour_profile section (vsf 0.9.2) so a display-referred ingest could store a JPEG's sRGB-encoded values as-is and let the *reader* linearize. No writer took that path: opsin linearizes at ingest and stores linear planes, so every writer emits `linear` and nothing anywhere reads it back — the only reader is vsf's own parser, round-tripping it. Its own doc calls it "the transfer the *samples* carry," which is a property of the plane, stored on the entry. That misplacement is the reason identity entries kept being invented: they were the only place to say "this plane is linear." It is superseded by the plane's `encoding` (implemented 2026-10-09, [`image.md`](image.md)); the entry field is still written and read for the legacy section and ignored. The path it was designed for, a JPEG's values stored as they came, exists since format v10, on the plane where it belongs: `primaries` and a source `encoding` ([below](#untouched-sources-and-prior--implemented-2026-10-20)). An encoding is not a characterization, so it needs no entry and no tier.

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
| `medium` | `peak_area`, `peak_luminance` | `t_f5 [n]` each, emissive only: a white window's fraction of the screen on black → the luminance it reached, cd/m², at the standard window sizes (1, 2, 5, 10, 25, 50, 100 %). An OLED or a local-dimming LCD reaches its peak only in a small window; this curve is how a renderer knows what a given picture can have ([`output.md`](output.md#peak-brightness-depends-on-how-much-is-lit)). Proposed 2026-10-20; the arithmetic is implemented (`colour::display::abl_peak`, `abl_headroom`) | proposed |
| `medium` | `peak_sustained` | `t_f5 [n]`: the same windows after the panel's long-duration limiting; absent ⇒ unknown | proposed |
| `plane.*` | `maker_crop` | `[x, y, w, h]` in sensor pixels: the maker's recommended crop (DNG `DefaultCropOrigin`/`Size` offset by `ActiveArea`, or a body's published crop). A **suggestion**, never applied — the plane and the view show everything recorded, masked borders included; opsin's crop tool starts here. Technical by nature; distinct from the `crop` op, which is the operator's | **implemented 2026-10-09** on the plane ([`image.md`](image.md#planetrackseqname)) |

### DNG carriage — decided 2026-10-08

Implemented in opsin (`src/idt.rs`), Nick: "I'm down with your recommendation." A DNG cannot say what an IDT claims, but it can carry one so that every reader applies it, and it can carry a note beside it that says what it is.

0. **The matrix goes in `CameraCalibration1/2`**, the specification's own per-unit slot, which every DNG reader multiplies onto the ColorMatrix: `XYZ→camera = CameraCalibration × ColorMatrix`. The paste writes `CC_i = M × CM_i⁻¹` per slot, so the product is the IDT exactly at each slot's illuminant. The file's own `ColorMatrix1/2` is untouched — the factory matrix survives. `CameraCalibrationSignature` and `ProfileCalibrationSignature` are set equal (`VERICHROME <class> <tier> target#<serial> <scanned>`), so a reader using the file's embedded profile applies the calibration and one that swaps in its own profile sees no match, ignores it, and falls back to its own per-model matrix — never our correction on someone else's base.
1. **The claim goes in XMP** (tag 700), namespace `https://verichrome.cc/ns/idt/1.0/`, prefix `verichrome:` — `IdtClass`, `IdtTier`, `Observer` (the matching functions the solve targeted: a DNG assumes CIE 1931, chameleon solves for CVRL 2006), `Illuminant`, the nine exact rationals as `Matrix` (so copy out of a pasted file is verbatim, not `CC × CM` rounded), `TargetType`, `TargetSerial`, `TargetCalibrated`, `Scanned`, `Source`, `Signature`, `Pasted`. An existing packet is kept byte for byte and the description added beside it; a re-paste replaces its own description.
2. **No original byte moves.** The new tags need a bigger IFD0, so the paste appends a copy of IFD0 with the added entries at the end of the file and repoints the header's one 4-byte offset. `OriginalIFD0` and `OriginalLength` in the XMP make undo (`opsin --unpaste-idt`) a 4-byte write and a truncate — byte for byte the file it was, even after several pastes.

A reader without the XMP sees an ordinary calibrated DNG. opsin reads all three: it elects a `VERICHROME`-signed calibration as `unit` / `relative` (or whatever the XMP says — a factory matrix pasted between frames of one camera records itself as `model` / `absolute`), and applies `CameraCalibration × ColorMatrix` on every DNG regardless, as the specification requires.

## Proposed additions — 2026-10-12: the medium response

Status: **proposed**. Fixed by [`output.md`](output.md#a-mediums-response-control-values-to-spectra); extends the `medium` section above. A medium's response is the camera response's mirror: control values in, spectrum out, stored as a K-dimensional table of spectra (or a model that predicts one).

| where | field | what | status |
|---|---|---|---|
| `medium` | `colorants` | `a` nl-joined: one name per control axis (`C M Y K`, eight inks, a spot ink, `R G B` drive levels for a display) | proposed |
| `medium` | `axes` | `t_f5 [Σnₖ]` + `axis_counts t_u5 [K]`: the grid nodes along each axis, coverage or drive in 0..1, non-uniform allowed | proposed |
| `medium` | `response_start`, `response_step` | the band grid, as the camera response's | proposed |
| `medium` | `response` | `t_f5 [n₁ … n_K, B]`: reflectance factor (reflective), transmittance (transmissive) or radiance (emissive) at every node. **Values above 1 are kept** | proposed |
| `medium` | `primary_spectra`, `primary_curves`, `black_spectrum` | the additive form for an emissive medium: `t_f5 [K, B]` primary spectra at full drive, `t_f5 [K, n]` drive-to-linear curves, `t_f5 [B]` emission at zero drive. Used instead of `response` for any display that passes an additivity check; `response` remains for displays with drive-dependent primaries or added processing | proposed |
| `medium` | `emission`, `excitation` | `t_f5 [n₁ … n_K, B]` and `t_f5 [B]`: the separable fluorescence model, M1 minus M2 per node, one excitation curve for the substrate. Optional | proposed |
| `medium` | `donaldson` | `t_f5 [n₁ … n_K, B, B]`: the full bispectral matrix per node, for whoever has a bispectrometer. Optional, supersedes the separable pair | proposed |
| `medium` | `model` | `a` open: `sampled` (the table was measured), `ynsn` (Yule–Nielsen spectral Neugebauer; then `primaries` + `yn_n` replace `response`), `cellular`, `km` (Kubelka–Munk) | proposed |
| `medium` | `condition` | `a` closed: `M0`, `M1`, `M2`, `M3`, the measurement condition (ISO 13655); required with `emission` | proposed |
| `medium` | `identity` | `a`: ink set, substrate, screening, press or printer, in words; the medium's equivalent of a camera's make and model | proposed |
| `medium` | `icc` | `v('b')`: an imported ICC output profile, verbatim. Its `A2B` tables render at tier `model`, D50 and CIE 1931 only; its `B2A` tables are a separation and are used for nothing else | proposed |
| `plane.*` | `role = ink` | a new plane role: K channels named by colorant, coverage, `black` 0 and `white` full. Rendered only through a `medium`; never blended with light planes | proposed |

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

## Untouched sources and prior — implemented 2026-10-20

Format v10. A plane can hold a source's samples exactly as written, and says what was done to them before the file existed. Decided with Nick that day: keep a source's samples untouched and change VSF to accommodate them. The reasoning is in [README](README.md#encodings-are-not-transforms).

| where | field | what | status |
|---|---|---|---|
| `plane.*` | `primaries` | `a` closed: `vsf` \| `srgb` (BT.709 primaries, D65) \| `display_p3` (DCI-P3 primaries, D65) \| `adobe_rgb` (D65) \| `prophoto` (ROMM RGB, D50) \| `rec2020` (BT.2020, D65). **Absent ⇒ `vsf`**, and written only when it is not. Anything but `vsf` requires a planar plane of three channels | **implemented** (`vsf::visual::Primaries`) |
| `plane.*` | `encoding` | gains a source's own transfers, each named for the specification that defines it: `srgb` (IEC 61966-2-1; also Display P3's), `bt1886` (the BT.709/BT.2020 SDR reference display, a 2.4 power), `adobe_rgb`, `prophoto` (1.8 with a linear toe), `pq` and `hlg` (BT.2100, diffuse white at 1.0 per BT.2408). A source's codes run 0 to 2ⁿ − 1 with white at the top code; VSF's own (`linear`, `gamma2`, `g2t`) keep ×2ⁿ, bucket bottom. The flat `image` section, which has no primaries, takes VSF's own only | **implemented** (`Encoding::light`) |
| `plane.*` | `prior` | `a` closed, an IDT class: what was done to the samples before the file and is not recorded in it. Absent ⇒ nothing; `technical` ⇒ mechanics only (demosaic, a frame merge); `creative` ⇒ a look (white balance, tone mapping: every camera JPEG). Mixed steps take the weakest claim; a writer that cannot tell writes `creative` | **implemented** |
| `graph` op | `decode` | Technical, arity 1, no params: its operand, a plane in foreign primaries, decoded by the plane's encoding and taken into VSF RGB by its primaries' matrix. Placed after `black_white` and before `idt` in a still's default graph. The plane says which primaries; the op says where in the program; the two cannot disagree | **implemented** (`OP_DECODE`) |
| `colour::legacy::constants` | `DISPLAY_P3_2VSF_RGB`, `PROPHOTO2VSF_RGB` and inverses | generated like sRGB's and Adobe RGB's: thru 1931 XYZ, colorimetric, with no chromatic adaptation, so a D65 or D50 white lands where that light sits in VSF RGB | **implemented** (`tools/generate_constants.rs`) |
| importer | `prior` from evidence | an importer sets `prior` from what it can show: `creative` for a camera JPEG or when nothing says otherwise; `technical` or absent when a signed manifest (C2PA's action list) or the writer's own knowledge says so. The evidence itself goes to provenance | opsin, next |
| `Plane::encoded` | — | the constructor for an encoded source: planar RGB, `black` 0, `white` the top code, role `display`, name `main`, `prior = creative` | **implemented** |

What `prior` deliberately does not hold: the device (that is `capture`), the event and its evidence (provenance: `attest`, `witness`, `run` from `lumis/docs/provenance.md`, and per-generation `change`), and whether the scene was real. Provenance can inform `prior`; `prior` never feeds provenance. Two tests keep the split honest: with provenance stripped, a reader still renders correctly and says what it cannot vouch for; with colour ignored, signatures still verify and the history still reads.

A derived picture, such as a JPEG exported from a signed lumis capture, is `prior = creative` in its own file even though its rendering is fully recorded in the parent: `prior` means "not recorded *in this file*". Provenance can point at the parent plane by its hash, which tells a reader where the steps are written down without changing the class.

## Migration

About ten 16-bit VSF images exist as of 2026-09-22, all written by opsin with linear planes. Under the absence rule their integer samples would read as gamma 2. **Resolved 2026-10-09 without a table:** the legacy `spectral_image` section had one writer and it wrote linear, so `Visual::read` adapts a legacy document to a one-track visual whose plane is `linear` ([`image.md`](image.md#absence)). New files carry `encoding` explicitly — which is the reason it is a field and not only a default. The visual container itself is specified in [`image.md`](image.md); this page remains the catalogue of fields and proposals.
