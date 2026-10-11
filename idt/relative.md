# The Relative IDT

<p align="center"><img src="images/relative.webp" width="560" alt="Relative IDT (DSR) — the object, independent of lighting"></p>

A Relative IDT tells you what the object *is*. The illuminant is solved out spectrally, so the pixel reports the surface's own properties, invariant of what was lighting it. This is Direct Scene-Referred — DSR — and it is what chameleon produces from a target scan.

Where an [Absolute IDT](absolute.md) is faithful to the light, a Relative IDT is faithful to the object. Same scene, same camera, two different true statements.

## The solve

The camera sees, per channel, the integral of reflectance × illuminant × sensitivity. A calibration target carries patches whose reflectance spectra are known. Shoot the target under the scene's illuminant and the solve has enough to separate the three: the reflectances are given, the sensitivity is the camera's, and what remains is the illuminant — which is then divided out. The result maps counts to **reflectance in VSF RGB**: what the surface would return under Illuminant E, VSF RGB's white.

That is why a Relative IDT built as a single matrix is *per lighting condition*: it divided out one specific illuminant, and applied to a scene under another it is wrong by the ratio of the two. The representation below removes that limit by keeping the illuminant as its own term. An entry's `illuminant` field remains for selection among stored matrices; it is **never** used for chromatic adaptation. Adaptation is a [Creative](creative.md) op, and a bad one.

## The representation — decided 2026-10-10

A Relative IDT is **a spectral response and an illuminant ratio**, not a matrix. The matrix is derived.

```
P₀_c(λ)   = S_c(λ) · E_ref(λ)        the camera's channel c under the calibration light — stored once per sensor
ρ(λ)      = E_shot(λ) / E_ref(λ)      the shot's light relative to that light             — per shot; 1 when absent
counts_c  = g · Σ_λ P₀_c(λ) · ρ(λ) · R(λ)                                                   what a surface R produced
```

`P₀` has as many channels as the camera (three for a Bayer set, seven for a seven-channel camera); the output has as many as the observer (three for CIE 1931 or CVRL 2006, four for a tetrachromat). The IDT at render time is the `N × K` matrix that best solves `Σ P₀·ρ·R → Σ CMF_E·R` over a reflectance ensemble in a perceptual metric, with the overall scalar `g` factored out as `sensitivity` exactly as below. Nothing in this needs an absolute illuminant: `E_ref` is whatever lit the target, and it cancels — a shot's `ρ` is relative to the same light the response is.

What is stored:

- `P₀` — the response curves, on the target's band grid, **one common scale for all channels** (their relative heights are information: they are the channels' sensitivities under the reference light, with whatever per-channel gain the readout applied). Recovered from the scan by a regularised non-negative solve; the regulariser is a stated choice (`response_prior`), because 21 broad patches cannot distinguish a single-humped curve from a two-humped one and the **shortest curve** (least path length) is the one a dye set can be, where a quadratic smoothness prior is not.
- `response_light` — the identity (content hash) of the scan that is the reference light. Two frames with the same `response_light` share a reference, so their `ρ` compare.
- the scan's own best-fit matrix, as today — now a **cache**: what a reader without curves uses, and what a reader with curves checks itself against (on the scan frame the derived matrix and the direct fit differ by 2–3 % of white; a larger gap means a broken `ρ` or a wrong prior).
- per shot, `ρ` — a Technical op on the view graph (`relight`). Footage from the scan's own light has `ρ = 1` as a fact: the target's greys told us exactly what grey is under that light, so that is the common case and it needs nothing more. A different light gets its own target scan (exact, and well posed because `S` is already known), or a measured SPD divided by the reference when a spectrometer reading exists (`capture.spd`). Nothing is ever scaled off a single patch, and a camera's as-shot white balance is not an input: white balance is a Creative guess, and this is a measurement.

What follows from it:

- **Relighting is a change of `ρ`**; the desired output light is a change on the observer side. Neither touches the stored response.
- **The observer is a render-time choice**, since the curves are spectral.
- **Absolute is the same object with one more fact.** A measured `E_ref` turns `P₀` into `S` and the scalar into radiance; every Relative IDT ever made against that reference upgrades retroactively, which is what `response_light` is for.

The study that settled this (chameleon `spectral.rs`, three Sigma fp frames and a Canon, 2026-10-10): a transform built from recovered curves over an ensemble that never saw the patches matches the direct 21-patch fit on held-out patches within 0.1–0.4 % of white — the curves carry everything the chart knows — and, as a study result only (not a paste path), a new frame's transform from three neutral patches alone landed within 0.7 % of white of its own full-chart fit under a broadband light — the response carries the light's whole effect once any neutral is measured. The two routes disagree by 2–3 % of white on the chart and 3–4.5 % on spectra the chart lacks: the price of the chart's blind spots, paid either way, and the gap the stored cache makes visible.

## Where the scale comes from

Not from the white patch. The white patch on the target is around 80% reflectance and not especially neutral. Not from the 50% patch either, though it is close. Not from any single patch.

Every patch on the target has a measured reference value — none of them exactly 18% or 50% or any round number — and the solve fits the transform against all of them at once. The overall scale of that transform is a fit output: the best-fit scalar over every patch, weighted by however the solve weights them. It falls out of the solve for free, which is the difference from Absolute, where the scale must be anchored to an emitter's measured output.

As with Absolute, the scalar is factored **out** of the matrix and stated as `sensitivity`. Its unit is reflectance: `sensitivity` is the linear count at white level expressed as a fraction of a perfect diffuse reflector under the solved illuminant, and 1.0 in the output is that reflector. The matrix is chromatic; the scalar is magnitude. Two fields, one fact each, no way for them to disagree.

## The field set

Everything Absolute needs except the emitter's absolute level, plus the solve's own inputs so any reader can audit or refit:

```
spectral_image
  samples, black[k], white[k], encoding, channels[k].curve      as for Absolute

colour_profile   target = vsf_rgb
  entry
    class                    relative
    tier                    unit
    matrix                   camera → VSF RGB, normalised
    sensitivity[k]           counts at white level, as a fraction of a perfect diffuse
                             reflector under the solved illuminant
    illuminant               which illuminant this entry was solved under — SELECTION, not adaptation
    unit                     reflectance
  patches                    (camera-space patch means, reference values) — the solve's inputs
  cal
    target_type, target_serial, timestamp
```

`patches` is the raw question the entry answers, kept verbatim. A reader with a better solver, or a different weighting, refits from it.

No `capture` section is needed. Reflectance is a ratio and exposure cancels — provided the frame was shot at the settings the target was, or the sensitivity is rescaled thru `gain`. A Relative IDT solved at one exposure and applied at another is off by exactly that ratio, which is a Technical correction, not a colour one.

## The lens is in the fit

A target shot thru the lens characterises camera *and* lens together. Lens coatings differ per wavelength — a warm lens and a cool lens at the same T-stop deliver the same luminance and different colour — and the solve absorbs that. Which is precisely why the profile is fingerprinted to the sensor and the lens, and why a paste onto a different lens is refused unless forced, with a warning. The transform is only true for the glass it was measured thru.

## The two-camera test applies

Two cameras, two lenses, same target, same T-stop, same time: both Relative IDTs, done right, produce the same reflectance values with no adjustment. Everything in the [Absolute test](absolute.md#the-two-camera-test) carries over — spectral mismatch off the fit set, off-axis shading, linearization, the emitter's stability during the scan — with one item removed: the emitter's absolute level does not matter, because it was divided out. Its *spectrum* still does.

## What Relative is not

It is not radiance. The illuminant is gone and cannot be recovered from the entry alone. If a file needs to answer both "what was the light" and "what was the object," it carries both an Absolute and a Relative entry — they are the same solve with the illuminant kept or removed, and the profile's ordering says which the reader should prefer.

And it is not white balance. White balance is a diagonal scale in camera RGB that removes the overall cast and gets individual object colours wrong, because a diagonal is not a spectral solve. It is the naive approximation of what DSR does properly, and the format files it under [Creative](creative.md) where it belongs.
