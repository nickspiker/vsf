# The Absolute IDT

<p align="center"><img src="images/absolute.webp" width="560" alt="Absolute IDT — the scene as captured, ambient cast preserved"></p>

An Absolute IDT tells you the light that actually left the scene. Not what the object would look like under a better lamp — what the photons were, including everything the illuminant did to them. It is a straight inversion, cast preserved, and it is the only class whose output is a physical quantity you could check with an instrument.

Not always aesthetically pleasing. Always true.

## What it is, precisely

The same solve as a [Relative IDT](relative.md), with two differences:

0. **The illuminant is not divided out.** Relative solves for reflectance. Absolute solves for radiance. The cast stays.
1. **The scale is anchored to a physical quantity**, not to a target's reflectances.

Which means an Absolute IDT needs exactly one input a Relative one doesn't: the **absolute level of the emitter** the calibration was shot under. Know the emitter's spectral irradiance at the target — a spectroradiometer, or a lux reading with the SPD's shape — and the radiance leaving each patch is known absolutely. The solve then yields counts-per-radiance instead of counts-per-reflectance. That is the entire difference in inputs.

The "spectral" part is the channel sensitivity curves. Luther–Ives fails for every real camera, so a 3×3 matrix is always a fit over *some* set of spectra. The curves are the characterization; the matrix is the cached fit. Store the curves and any reader can refit for a different spectral set — the same audit principle the `colour_profile` section already runs on for DNG matrices and target patches.

**There is no illuminant anywhere in an Absolute IDT.** The data is the light. What lit the scene is scene metadata, not part of the transform.

## The strict field set

Every field here is present because the [two-camera test](#the-two-camera-test) fails without it. Field names are as they stand or are proposed in [`fields.md`](fields.md); status is marked there.

```
spectral_image
  samples                    counts, in any range you like
  black[k], white[k]         per channel, in counts
  encoding                   the sample encoding — gamma 2 per colour.md
  channels[k].curve          relative spectral sensitivity, peak-normalised

colour_profile   target = vsf_rgb
  entry
    class                    absolute
    grade                    unit
    matrix                   camera → VSF RGB, normalised — scale factored OUT
    sensitivity[k]           sensor-plane exposure at white level, per channel
                             J·m⁻², monochromatic-equivalent at the curve's peak
    gain_ref                 the gain sensitivity was measured at
    unit                     what 1.0 means in the output — see units.md
  cal
    target_type, target_serial, timestamp
    emitter                  which source, and its measured absolute output

capture
  exposure_s
  t_stop
  gain                       as the sensor reports it
  shading                    per-lens flat-field; centre-only without it
```

### `sensitivity` is the load-bearing field

It is defined against **white level, not sensor saturation** — so it is range-independent by construction. Store the data at one-sixteenth scale with `white` at the container maximum and `sensitivity` is simply sixteen times larger. The container range is a storage decision; the scalar says what the range means. This is `H_sat` from ISO 12232, made per-channel, made radiometric, and honest about what "white" is.

It is factored **out** of the matrix. If the scalar is a required, auditable field, the matrix must not also carry it in its overall magnitude, or the two can disagree. The matrix is chromatic; the scalar is magnitude.

## The read chain

A reader computes, per channel:

0. **Linear counts.** Undo `encoding`, subtract `black`, divide by `white − black`. Call it `n`. It lives in [0, ∞): values above 1 are real and preserved — a specular past white is data, not an error.
1. **Sensor-plane exposure.** `H = n × sensitivity × (gain_ref / gain)`.
2. **Scene radiance.** `L = 4 T² H / (π t)`, on-axis, after `shading` if present.
3. **VSF RGB.** `[R G B] = matrix × [L₁ L₂ L₃]`. Absolute radiance in the three primaries. Done.
4. **Display.** Not part of the IDT. Whatever is applied to make radiance viewable is a [Creative](creative.md) layer, and it is recorded as one.

Step 4 is what dissolves DNG's `BaselineExposure`. The IDT produces radiance; "how bright to show it" is a look, and the format has a place for looks that is not the characterization.

## The two-camera test

Two distinct cameras, distinct lenses, same target, same T-stop, same exposure time. If both Absolute IDTs are right, the outputs **match with no adjustments**.

Step 2 gives each camera the same `L`. Every difference between them — pixel pitch, quantum efficiency, fill factor, gain, container range, bit depth — is absorbed by steps 0 and 1, because all of those change counts without changing energy. Step 3 gives the same VSF RGB on the fit set.

This is not merely a check. Everything that would make it fail is, by definition, something the IDT must carry — so the test *is* the specification of the field set. What it cannot absorb:

0. **Spectral mismatch.** The fit is to the target, so both cameras agree on the target. For spectra outside the fit set they diverge, because two different curve sets cannot both be a linear transform of VSF RGB's primaries. Full curves narrow it. Nothing closes it. "Done right" means right on the fit set; that is physics, not a defect.
1. **The lens is part of the spectral system.** T-stop equalises broadband transmission, which is why it is T and not f. It does not equalise transmission per wavelength: coatings differ, and a warm lens and a cool lens at the same T-stop deliver the same luminance and different colour. A calibration shot thru the lens absorbs that into the fit — which is exactly why changing the lens invalidates the profile.
2. **Off-axis.** T-stop is an on-axis number and vignetting is per-lens. Whole-frame match needs `shading`; without it the test holds at the centre.
3. **Linearization.** The scalar is a linear-domain quantity. An encoding not undone, or counts above the sensor's linear range, break the match at some levels regardless of the scalar. This is why `encoding` is a required field on the plane rather than a note on an entry.
4. **Both halves of exposure.** Same T-stop *and* same time. Shutters carry a few percent of tolerance; that is the floor on "no adjustments."
5. **The emitter.** It has to stay fixed. LED warm-up and lamp ageing move the absolute level, and every Absolute scalar is only as good as the source it was measured against. That is why the emitter is recorded in `cal` — it is provenance, the same as the target serial.

Handle all six and the expectation is a few percent in luminance at the centre on the fit set, with colour agreement on the target to within the fit residual. Off-target spectra is where two cameras show their difference, and no scalar fixes that.

## Two honest degradations

- **No curves** — a magic-9 fit alone. `sensitivity` can only be defined at the emitter's SPD, so the IDT is absolute *for spectra near the fit set*. Still `absolute`, still `unit`; `curve` being absent tells the reader the bound.
- **No `capture` section.** The chain stops at step 1: sensor-plane exposure, not scene radiance. Still absolute, still comparable across cameras at the same settings — just not across settings.

## ISO is derived, not primary

"Clip equals ISO" has been formally defined for digital: **ISO 12232** saturation-based speed,

    S_sat = 78 / H_sat

where `H_sat` is the sensor-plane exposure in lux·seconds that just clips. The 78 is the photographic convention that mid-grey sits at 10/S lux·s plus √2 headroom above diffuse white, so grey lands near 12.7% of saturation. This is the number DxOMark publishes as "measured ISO."

It is almost never the number on the dial. The same standard defines **SOS** — the exposure that renders 18% grey to sRGB 118 in the maker's JPEG — and **REI**, the maker's recommendation. Makers label with those. Both are statements about a rendered output and include the maker's headroom decision, typically ⅓–⅔ of a stop and different per brand, so "ISO 100" on two bodies is two different clip points. For raw data SOS is meaningless; there is no sRGB 118 in a raw file.

Even `S_sat` summarises three channels under one daylight illuminant into a single V(λ)-weighted number, and clip is per-channel — under any real illuminant one channel saturates first. Above base ISO the ADC clips well before the pixel does; the dial is gain applied *after* the photons, in known steps.

So ISO is not the anchor. The emitter is. Define `sensitivity` radiometrically per channel at a recorded gain, and *derive* `S_sat` as a reported convenience — useful for cross-checking against the dial, and it tells you the maker's headroom. `capture.gain` records the dial value, because the analog steps are clean and the ratio to `gain_ref` is all the chain needs.

## The DNG lesson

DNG has one exposure concept, `BaselineExposure`, and its own spec describes it as producing "brighter default results" — a look. The same tag is also where a genuine calibration normalisation ends up, because there is nowhere else. One slot, two different claims, and a reader cannot tell which it holds.

The cost is not theoretical. At least one major grading application applies `BaselineExposure` at decode and clamps to the container before anything downstream can see the result, so any positive value destroys every sample above white — permanently, before the node graph. If the tag held a preference, that cost a preference. If it held a measurement, it destroyed data.

The Absolute IDT cannot have this problem, because it has no slot in which a look could be mistaken for a measurement. The sensitivity is inside the characterization and is Technical. Anything on top is Creative and is labelled so. A reader that clamps a Creative op has lost a preference, and the file says exactly that.

## Open decisions

0. **What 1.0 means.** VSF RGB output is floating point, so this is only a scale. SI — W·sr⁻¹·m⁻²·nm⁻¹ at each primary — is the honest interim; values will be small and that is fine. The intended home is the base-unit system in [`units.md`](../units.md), once it carries symbols.
1. **Where `shading` lives.** It is a tensor per lens. Its own section, or a reference to a sidecar.

Everything else in the chain is determined. The one thing to resist is any field that makes step 4 implicit. The moment a viewer must guess how bright to show absolute radiance, a look has leaked into the measurement, and that is the ACES mistake again.
