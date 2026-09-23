# The Relative IDT

<p align="center"><img src="images/relative.webp" width="560" alt="Relative IDT (DSR) — the object, independent of lighting"></p>

A Relative IDT tells you what the object *is*. The illuminant is solved out spectrally, so the pixel reports the surface's own properties, invariant of what was lighting it. This is Direct Scene-Referred — DSR — and it is what chameleon produces from a target scan.

Where an [Absolute IDT](absolute.md) is faithful to the light, a Relative IDT is faithful to the object. Same scene, same camera, two different true statements.

## The solve

The camera sees, per channel, the integral of reflectance × illuminant × sensitivity. A calibration target carries patches whose reflectance spectra are known. Shoot the target under the scene's illuminant and the solve has enough to separate the three: the reflectances are given, the sensitivity is the camera's, and what remains is the illuminant — which is then divided out. The result maps counts to **reflectance in VSF RGB**: what the surface would return under Illuminant E, VSF RGB's white.

That is why a Relative IDT is *per lighting condition*. It divided out one specific illuminant. Apply it to a scene under another and the answer is wrong by the ratio of the two. A profile may therefore carry several Relative entries, one per illuminant it was solved under, best first; the entry's `illuminant` field is there so a reader can select the matching one. It is **never** used for chromatic adaptation. Adaptation is a [Creative](creative.md) op, and a bad one.

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
    grade                    unit
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
