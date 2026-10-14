# Layers — the K-channel compositing model

Status, 2026-10-12: **design**, agreed in principle with Nick; the op inventory below is the worklist and each filter gets taken piece by piece. This page binds what the format can say; `image.md` holds the wire fields (`graph`, `plane`), this page holds the rules an evaluator follows. Nothing here is implemented yet beyond the still's default graph.

## The data: planes with K channels, K unconstrained

A plane is K channels of linear samples with a name and, when characterized, a spectral sensitivity curve per channel (`image.md`, `channel_names`, `curve_*`). K is whatever the source had: 1 for a Monochrom, 3 for a Bayer camera, 4 for RGBW or an RGB + alpha composite, 25 for an LED multispectral composite, 1000 for a hyperspectral cube. **Nothing in the pipeline assumes a count, and no count implies a meaning**: four channels is not CMYK, it is four channels whose names and curves say what they are. (Photoshop's header allows 56 channels but a mode field decides what the first few mean and everything past that is alpha or spot; readers that infer "4 = CMYK" are reading the mode, not the data. VSF has no mode field, on purpose.)

Every sample carries its scale with it: a plane's `black` and `white` per channel are the only statement of what a count means. There is no `255`, `32768` or `1.0` anywhere in the model; those appear exactly once, at import and export, in the tables at the end of this page.

## Where spectral lives

Spectral data derives transforms; it is not evaluated per pixel. The characterization's response curves × the chosen observer × the desired illuminant give one `N × K` matrix for an N-channel observer (3 for CIE 1931 or CVRL 2006, 4 for a tetrachromat; `relative.md`, "The representation"). The relight ratio is a curve; it changes the matrix. Everything per-pixel is then a matrix on a K-channel plane, which is what opsin renders today.

And the per-pixel operations that *are* spectral need no special machinery: light adds per band and a filter multiplies per band, so **add and multiply per channel in linear light are the spectral operations when the channels are bands**, and they are the physically correct ones when the channels are a camera's three. K = 31 costs 31/3 the arithmetic of K = 3 and nothing else.

## Two kinds of operation

The IDT taxonomy draws the line (`README.md`): Technical ops make no claim and have no taste; Creative ops have one.

**Physical ops** act on K channels in linear light and are defined by physics: `add`, `multiply` (transmission), `mix` by a mask plane, `exposure` (a scalar, Creative by the taxonomy but channel-blind), `relight`, `black_white`, `crop`, `orientation`, `scale`, `mean`/`median`/`sum`/`min`/`max` over frames, `sub`, `div`. They never need to know what the channels mean and they never leave K channels.

**Observer ops** are defined by how something *looks*: hue, saturation, "color", luminosity, white balance, a tone curve on perceived lightness. They are defined on the N-channel rendered view, not on the K-channel data, so they go K → N through the observer and operate there. They cannot come back: N → K has no inverse once K > N, and an op that pretended otherwise would be inventing data. So observer ops live on the **view side of the graph**, after `idt`, and the output of the graph past the first observer op is N-channel. This is the line Photoshop never drew; it is why Photoshop had to hard-code meaning onto a channel count.

**Ink planes are neither.** A plane of colorant coverages (`role = ink`) is device data, not light: it is rendered only through a medium's response ([`output.md`](output.md#a-mediums-response-control-values-to-spectra)) and never enters a blend with a light plane. The op that produces one, separation, is Creative and comes last.

A reader strips Creative ops for a technical export (`image.md`); with this split that is well defined: everything up to and including `idt` is K-channel and physical, and the Creative ops after it are arity-1 and can be dropped one by one.

## Blending space is a parameter, applied as a wrapper

Photoshop blends on gamma-encoded values because its documents are gamma-encoded; Krita blends in linear light; every compatibility argument is about which curve the arithmetic happens under. In this model that is not a property of the document or of the engine. It is **one parameter on a blend op**, `space`, and the evaluator applies it as a wrapper: encode both operands with the curve, run the op's arithmetic, decode the result. For `space = linear` the wrapper is the identity and costs nothing. For an op that is linear in its operands (`add`, `mix`, `mean`) the wrapper cancels exactly and the parameter is irrelevant; for the ones that are not (multiply, screen, the light family, the dodge/burn family) it is the whole difference between Photoshop's answer and the physical one.

```
blend(mode, space, a, b)  =  decode_space( MODE( encode_space(a), encode_space(b) ) )
```

`space` is one of `linear` (the default, the physical answer), `gamma2` (VSF's own integer encoding), `srgb`, or `document` (whatever encoding the layer was imported with — the value that reproduces a PSD exactly). A PSD imported with `space = document` on every blend renders as Photoshop rendered it; flip the parameter and the same graph renders physically. The user chooses; the file records the choice per op; nothing is baked.

## Op inventory: the worklist

Each op gets a row here as it is specified. Photoshop's 27 blend modes are listed because the import has to map them, and because deciding which side of the line each falls on is most of the work.

| op | kind | channels | blend-space sensitive | notes |
|---|---|---|---|---|
| `over` (Normal) | physical | K + alpha | no | Porter–Duff over with straight alpha; alpha is a plane like any other |
| `dissolve` | physical | K + alpha | no | stochastic coverage from alpha; needs a seeded noise source stated in the op so a render is repeatable |
| `multiply` | physical | K | **yes** | transmission in linear; Photoshop's result under `document` |
| `screen` | physical | K | **yes** | `1 − (1−a)(1−b)` on normalized light |
| `add` (Linear Dodge), `subtract`, `difference`, `exclusion`, `divide`, `linear burn` | physical | K | no / yes | arithmetic per channel; `difference` and `exclusion` are encoding-sensitive, the rest linear |
| `darken`, `lighten` | physical | K | no | per-channel min / max; monotone curves commute with min/max |
| `overlay`, `soft light`, `hard light`, `vivid light`, `linear light`, `pin light`, `hard mix`, `color dodge`, `color burn` | physical form, Creative intent | K | **yes** | piecewise curves of the operands; defined by Photoshop's formulas, reproduced under `document`; under `linear` they are different looks and should say so |
| `hue`, `saturation`, `color`, `luminosity` | **observer** | N | — | defined in an opponent / lightness space of the rendered view; K → N first |
| `darker color`, `lighter color` | **observer** | N | — | compare by luminance of the rendered view |
| `pass through` | structure | — | — | a group that does not composite; a graph node with no op |
| `mix` (mask) | physical | K + mask plane | no | `a·m + b·(1−m)`; the regional edit primitive |
| `relight` | physical | K | no | the illuminant ratio, `relative.md` |
| `white balance` | observer | N | — | a diagonal in a display space; Creative, and named so (`creative.md`) |
| `exposure`, `dr_curve`, `curve` | Creative | K or N | — | `exposure` is channel-blind on K; a tone curve on lightness is an observer op |

Filters (blur, sharpen, noise, distort, …) are a separate list; each is a kernel on K channels in linear light unless it is defined on perceived lightness, and each gets its row when it is done.

## Integers: the 255 / 256 rules, stated once

Inside VSF there is no rule because there is no fixed white: every plane says its own `black` and `white`, in its own counts, and a sample's light is `(v − black) / (white − black)`. The madness lives at the borders, and these are the borders.

**Import from 8- and 16-bit display-referred sources (PSD, TIFF, PNG, JPEG).** The source's `white` is its code limit: 255 for 8-bit and 65535 for 16-bit, **PSD included**. Photoshop works internally in 0 to 32768 (15 bits plus one, so white is a power of two and 0.5 is an exact code), but its files store full-range 0 to 65535 and it converts on load and save. The internal range shows up only as quantisation: a 16-bit file saved by Photoshop holds about 32,769 distinct levels, odd codes do not survive, and so 16-bit PSD is not a lossless home for a 16-bit plane. Samples are decoded through the embedded profile's curve to linear (matrix/TRC profiles honoured, LUT profiles declined and the sRGB convention taken with that fact recorded; the opsin JPEG rule), and the plane is written with `black = 0`, `white = the code limit`, `encoding` stating what was done. 32-bit PSD is IEEE float, linear, 1.0 = diffuse white; it is the only PSD form that carries linear data and HDR, and it imports losslessly.

**Export to PSD.** A PSD is display-referred, so export is where translateration ends: the graph is evaluated to the export observer and space (sRGB, Display P3, Rec.2020, or linear with a linear profile), the chosen ICC profile is embedded, and:

| PSD depth | code for light `n ∈ [0, 1]` | notes |
|---|---|---|
| 8-bit | `n · 255`, **error-diffusion dithered** to the code, never truncated | the stack's rule for every 8-bit output (lumis, opsin); a plain round is a banding generator |
| 16-bit | `n · 65535`, dithered | the file's range; Photoshop requantises to its internal 0..32768 on load, so the export cannot be more precise than that |
| 32-bit | `n` as f32, linear, no transfer | the lossless path; use it when the receiver can |

Opacity: Photoshop's layer opacity is a byte, `255` = 100 %; fill opacity (`iOpa`) likewise; the UI's percent is `round(byte / 2.55)`. VSF ops carry opacity as a plain fraction and export rounds to the byte. Masks and alpha are planes with their own levels and export as 8- or 16-bit channels by the same table.

**What cannot go to PSD and what happens instead.** K > 4 colour channels: PSD has no home for them; export the rendered N-channel view, and write the extra planes as spot channels only when the user asks (a spot channel in Photoshop is a printing plate, not a band). Observer ops: PSD has blend modes, not a graph; ops with a PSD equivalent map (the table above), ops without one are **baked into that layer's pixels and the layer is marked** so the loss is visible. Generations: PSD has no history; the current generation exports. Lineage and signatures: dropped, and the export says so.

**Import from PSD.** The flat composite becomes a `preview`-role plane with lineage to the import; each layer becomes a plane (K = the mode's channels, plus alpha as a plane, plus the mask as a plane); groups become graph nodes; blend modes map by the table with `space = document`; adjustment layers, smart objects and type layers are kept **verbatim as a `foreign` blob** on the plane until an op exists for them, so a round trip loses nothing even where we cannot yet evaluate. That is photocraft's discipline (byte-exact round trip of what you do not understand) applied to a format that was designed for it.

## Where to start

0. This page: the op inventory is the worklist. Each filter gets its row — kind, channels, space-sensitivity — before it gets code.
1. The evaluator (`vsf::visual` M3): K-channel planes in f32 linear, the blend wrapper, the physical ops, `idt` deriving `N × K` from the response. One implementation, shared by opsin, fluor and lumis, so no two compositors disagree.
2. PSD and layered-TIFF **import** through photocraft's `psd` crate (MIT/Apache, clean-room from Adobe's spec, byte-exact round trip), behind an opsin feature: layers → planes and graph, unknowns → `foreign`, blends with `space = document`. Import first because it is lossless and because it produces the test corpus for the ops.
3. PSD **export** by the table above, with the two decisions it forces made explicit: dither on every integer export, and bake-and-mark for anything PSD cannot say.
4. Observer ops, one at a time, in a stated opponent space, on the view side only.

What this page does not decide, and should not yet: the opponent space for observer ops (candidates: MacLeod–Boynton on the chosen observer's fundamentals, which opsin's chart already uses), and whether `space = document` on an imported PSD should be the per-document default or per-op. Both are Nick's calls once the import exists and the comparisons can be looked at.
