# Input Device Transforms

An Input Device Transform relates the light a sensor captured to the values a file hands you. That is the whole definition, and it is enough to force a taxonomy: a transform either preserves what the light was, preserves what the object was, deliberately departs from both, or makes no claim about the scene at all. Those are the four kinds. There is no fifth that isn't one of them wearing a hat.

This directory is the specification of how VSF records them. It sits on top of [`colour.md`](../colour.md), which defines the colourspace every IDT lands in — VSF RGB, monochromatic primaries at 703/523/462 nm, Illuminant E white, gamma 2 — and on [`units.md`](../units.md) for the physical units an Absolute IDT ultimately speaks in.

## The four classes

<table>
<tr>
<td align="center"><img src="images/absolute.webp" width="360" alt="Absolute IDT"><br><b><a href="absolute.md">Absolute</a></b><br>the light, cast and all</td>
<td align="center"><img src="images/relative.webp" width="360" alt="Relative IDT (DSR)"><br><b><a href="relative.md">Relative</a></b><br>the object, illuminant divided out</td>
</tr>
<tr>
<td align="center"><img src="images/creative.webp" width="360" alt="Creative IDT"><br><b><a href="creative.md">Creative</a></b><br>a deliberate departure</td>
<td align="center"><img src="images/technical.webp" width="360" alt="Technical IDT"><br><b><a href="technical.md">Technical</a></b><br>no claim about how it looked</td>
</tr>
</table>

0. **[Absolute](absolute.md)** — a straight inversion. The pixel tells you the light that actually left the scene, including whatever the illuminant did to it. Faithful to the light.
1. **[Relative](relative.md)** — Direct Scene-Referred (DSR). The illuminant is solved out spectrally, so the pixel tells you the surface: what the object *is*, invariant of what was lighting it. Faithful to the object.
2. **[Creative](creative.md)** — a deliberate shift. White balance, a camera maker's look, a curve, a second exposure scalar. Faithful to a preference.
3. **[Technical](technical.md)** — mechanics and instrumentation. Orientation, crop, a stated sensitivity, a near-infrared or X-ray band remapped into the visible. Makes no claim about how the scene looked and expresses no taste.

Absolute and Relative are the two *characterizations* — the entries in a file's `colour_profile`. Creative and Technical are mostly *layers* — ops in the `view_transform` log, applied in order after characterization. The one crossover is the Technical sensitivity scalar, which belongs inside the characterization because without it a characterization is incomplete: a matrix fixes ratios, so chromaticity, and says nothing about magnitude.

## Class and grade are orthogonal

Every entry carries two independent facts. Its **class** says what kind of relationship it claims to the scene — one of the four above. Its **[grade](grades.md)** says how that claim was arrived at: measured on this unit, given by the manufacturer for the model, or implied by a format's convention. A DNG matrix is Absolute at Model grade. A chameleon target scan is Relative at Unit grade. An untagged JPEG read as sRGB is Absolute at Assumed grade.

They are kept apart on purpose. Grade exists so a guess can never silently read as a measurement. Class exists so a look can never silently read as a characterization. Both are the same principle — a weaker claim must not pass as a stronger one — pointed at two different axes.

## Absence is a statement

A `spectral_image` with no `colour_profile` is not uncharacterized. Its samples **are** VSF RGB, by specification. There is nothing to grade because nothing is being claimed beyond what the format already guarantees, and any reader that meets no profile renders thru the identity with Illuminant E as the white. See [`fields.md`](fields.md) for the defaults that absence implies, including the sample encoding.

This is not a convenience. It closes a hole: the alternative is an identity entry that must carry *some* grade, and every available grade misdescribes it — `Assumed` calls a definite fact a guess, `Unit` calls it a measurement. Absence is the only honest representation, and the format was designed so that it is also the default one.

## Why not ACES

ACES has a single, undifferentiated IDT slot — camera-native to ACES2065-1 — and nothing in it says what relationship the transform claims to the scene. Its own separate stage for looks, the LMT, does not fix this: in practice vendors ship their picture style *inside* the Input Transform, and the format has no vocabulary to say so. A characterization and a house look wear the same label.

Then its Reference Rendering Transform is a fixed, mandatory, opinionated film-like rendering that every image passes thru on the way to a display — a Creative transform living in the slot marked Reference. Its documented 1.x artefacts (saturated reds skewing orange, blue lights going purple) come from exactly that: a per-channel tone curve applied where a measurement should have been left alone. ACES 2.0 rewrote the output transform to fix them, which is a tacit admission the reference rendering was wrong for a decade.

ACES also has no grade axis, so a magic-9 scan and a guess are the same kind of object, and it is tristimulus all the way down, so the 1931 observer is baked into the encoding and cannot be updated after the fact — the problem [`colour.md`](../colour.md) exists to solve.

The four classes are not a variant of the ACES model. They are what ACES has one slot for.

## Pages

- [absolute.md](absolute.md) — the Absolute IDT: what it is, the strict field set, the read chain from counts to radiance, the two-camera test, and why ISO is derived rather than primary
- [relative.md](relative.md) — the Relative IDT: the DSR spectral solve, where its scale comes from, and what the lens has to do with it
- [creative.md](creative.md) — Creative layers: white balance as the naive DSR, camera looks, curves, and the rule for a second exposure scalar
- [technical.md](technical.md) — Technical ops: value-preserving mechanics, instrument remaps, and the line between them and Creative
- [grades.md](grades.md) — Unit, Model, Assumed: the trust axis, and the grade that was added and removed
- [fields.md](fields.md) — every field a VSF image carries today, the wire names, the defaults absence implies, and the proposed additions with their status
