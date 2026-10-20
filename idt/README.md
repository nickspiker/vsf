# Input Device Transforms

An Input Device Transform relates the light a sensor captured to the values a file hands you. That is the whole definition, and it is enough to force a taxonomy: a transform either preserves what the light was, preserves what the object was, deliberately departs from both, or makes no claim about the scene at all. Those are the four kinds. There is no fifth that isn't one of them wearing a hat.

The relation runs both ways. A medium — a print, a panel, a slide — turns the values back into light, and the same four kinds describe that transform too; what changes is which of them the medium can physically keep. A print is the Relative case made of paper. See [`output.md`](output.md).

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

## Class and tier are orthogonal

Every entry carries two independent facts. Its **class** says what kind of relationship it claims to the scene — one of the four above. Its **[tier](tiers.md)** says how that claim was arrived at: measured on this unit, given by the manufacturer for the model, or implied by a format's convention. A DNG matrix is Absolute at Model tier. A chameleon target scan is Relative at Unit tier. A RAW with no matrix at all carries the identity at Assumed tier. A JPEG carries neither: what it holds is an encoding, not a characterization (below).

They are kept apart on purpose. Tier exists so a guess can never silently read as a measurement. Class exists so a look can never silently read as a characterization. Both are the same principle — a weaker claim must not pass as a stronger one — pointed at two different axes.

## Absence is a statement

A `spectral_image` with no `colour_profile`, or a plane with no `characterization` and no `primaries`, is not uncharacterized. Its samples **are** VSF RGB, by specification. There is nothing to tier because nothing is being claimed beyond what the format already guarantees, and any reader that meets no profile renders thru the identity with Illuminant E as the white. See [`fields.md`](fields.md) for the defaults that absence implies, including the sample encoding.

This is not a convenience. It closes a hole: the alternative is an identity entry that must carry *some* tier, and every available tier misdescribes it — `Assumed` calls a definite fact a guess, `Unit` calls it a measurement. Absence is the only honest representation, and the format was designed so that it is also the default one.

## Encodings are not transforms

Three kinds of thing get called a colour profile, and they are told apart by what sits at each end:

| kind | maps | a real device? | carries | class | tier | in VSF |
|---|---|---|---|---|---|---|
| **capture** | light → counts | yes | the per-channel response, a cached matrix, the observer | the claim it makes: Absolute or Relative | yes | `characterization` |
| **encoding** | code values ↔ light | no: a definition | primaries, white, transfer | none | none | a plane's `primaries` and `encoding` |
| **medium** | drive values → light | yes | its kind, white and black, its response | the classes it can keep ([`output.md`](output.md)) | yes | `medium` *(proposed)* |

Capture and medium are the same object facing opposite ways: a device's response, measured, and so tiered. An encoding has no device in it. sRGB, Display P3, Adobe RGB, ProPhoto, Rec.2020 and VSF RGB itself are each a rule from code values to light, exact by their specification, so there is nothing to measure and nothing to tier, for the same reason absence needs no tier (above). Converting from one encoding to another is Technical, and colorimetric: sRGB's white is D65 light and lands where D65 sits in VSF RGB, a little bluer than VSF white. Making it neutral would be white balance, which is [Creative](creative.md#chromatic-adaptation-honestly-labelled) and is recorded as an op when someone wants it, never folded into the conversion. ICC has a profile class for these (`spac`, ColorSpace), and ISO 22028 calls them colour encodings.

A plane can keep a source's samples exactly as they were written: a JPEG's codes stay its codes, with `primaries` and `encoding` saying what they mean, and the graph's Technical `decode` op taking them into VSF RGB ([`fields.md`](fields.md#untouched-sources-and-prior--implemented-2026-10-20)).

### What came before the file

An encoding says what a code means. It says nothing about how the light became that code. A phone's JPEG went thru demosaicing, a frame merge, the phone's colour matrix, white balance, local tone mapping, sharpening and noise reduction before the file existed. None of it is recorded, and most of it cannot be undone. A graph reading `plane → decode → view` would show one Technical step and no Creative ones, and a reader would take the picture as faithful to the scene: a look passing as a measurement, by omission.

So a plane states, as a class, what was done to its samples before the file: `prior`. Absent means nothing was (raw counts, a drawing's own values); `technical` means mechanics only, such as a demosaic or a merge in the camera; `creative` means a look was applied, which is every camera JPEG, since white balance alone decides it. Where the steps mix, the weakest claim wins, and a writer that cannot tell writes `creative`, because absence would claim that nothing happened.

`prior` answers a colour question and stops there. Which camera made the picture is in `capture`. That the camera's processing happened, and the evidence for it (an EXIF tag, a signed manifest), is provenance (`lumis/docs/provenance.md`). Provenance can inform `prior`: a signed action list that names only a crop justifies `technical`. Whether a scene was real is a provenance question, not a colour one: a photograph of a screen showing a synthetic picture is a true measurement of the screen.

Output-referred encodings (ISO 22028's term) bundle one more thing: a reference display or print and its viewing conditions. sRGB is primaries, white and transfer plus an 80 cd/m² reference display. In these terms that is an encoding plus a medium, at whatever tier the specification gives it, and a picture tagged sRGB has already been rendered for that medium: its `prior` is `creative`.

## What a spectral profile is, and why

Almost every camera profile in use is a matrix. A DNG's `ColorMatrix`/`ForwardMatrix`, an ICC input profile and an ACES IDT all reduce to the same object: a 3 × K matrix fitted to a chart under a light or two, interpolated by colour temperature for anything in between, with look tables on top. It answers one question well: under *that* light, for *those* patches, which matrix lands closest. Ask it about another light or another observer and it can only guess, because what it was fitted from is gone.

A spectral profile keeps what it was fitted from. It stores the camera's response per channel per wavelength (`P₀`, [`relative.md`](relative.md#the-representation--decided-2026-10-10)), and the matrix becomes something a reader derives at render time. That buys:

- **Any light.** Relighting is a change of the illuminant ratio `ρ` in the model, not an interpolation between two fits by colour temperature.
- **Any observer.** CIE 1931, CVRL 2006 or a tetrachromat's four channels: the curves are integrated against whichever one the render asks for.
- **Any channel count.** Three channels, five or thirty-one go thru the same arithmetic. A five-channel camera needs no new kind of profile, just five curves.
- **An error bar.** The stored best-fit matrix is kept as a cache, and the gap between it and the matrix derived from the curves shows how much the chart could not see.
- **An upgrade path.** Measure the reference light once and every Relative profile made against it becomes Absolute.

It does not make a camera see more than it can. Two spectra that a camera's curves map to the same counts stay indistinguishable, and the derived matrix is still a best fit over a reflectance ensemble. What changes is that the fit's inputs travel with the file, so any reader can redo it.

DNG has no tag for a camera's spectral response. iccMAX (ICC.2) can carry spectral data; ICC v4, which is what ships, cannot.

## Absolute and Relative are ICC's intents, spectrally

The distinction is ICC's: absolute colorimetric keeps the white as part of the signal, relative colorimetric uses it as the reference. These two classes make the same choice at capture, about the light. They differ from ICC in two ways.

0. **Spectral, not tristimulus.** ICC normalises in 1931 XYZ. With three values a white cannot be divided out without a chromatic adaptation model, and Bradford, CAT02 and von Kries each give a different answer; that spread is the approximation showing. A Relative IDT divides the illuminant out per wavelength, which is exact and needs no adaptation model. Camera white balance is the same three-value shortcut, and is filed as [Creative](creative.md#chromatic-adaptation-honestly-labelled).
1. **A perfect reflector is the reference, not the medium's white.** ICC relative colorimetric normalises to the medium's own white, the paper as measured, so a bluish paper becomes neutral and the substrate leaks into the reference. Here the reference is a perfect diffuse reflector under the light: the light is divided out and every real surface keeps its own colour, the target's white patch and the paper included. On the way out, "relative" is one neutral scale onto the medium's white (Technical). Taking the paper's tint as neutral is a separate step, and it is Creative ([`output.md`](output.md#what-a-renderer-does-and-what-class-that-is)).

For an emissive medium the two references coincide, because a display's white is its light. On paper they do not, and this is where ICC's relative intent gets muddy: it mixes a property of the medium into what should be a property of the light.

## Relation to ACES

ACES gave the film industry a shared scene-referred interchange, ACES2065-1, and the Academy's procedure for building an IDT starts from the right input: the camera's measured spectral sensitivities, fitted over a training set of spectra (rawtoaces implements it). Where this format differs, it is on four points.

0. **The observer is fixed in the encoding.** ACES's primaries and white are specified as CIE 1931 xy chromaticities, and ACES2065-1 is defined relative to 1931 XYZ. An xy pair carries no spectrum, so a better observer such as CVRL 2006 cannot be adopted without redefining the encoding itself. VSF RGB's primaries are wavelengths, which any observer can evaluate ([`colour.md`](../colour.md#why-spectral-definition-matters)).
1. **The spectral sensitivities are not kept.** They are the input to the IDT fit, and the result travels as a matrix. A reader cannot re-derive the transform for another light or another observer, which is what a spectral profile is for (above).
2. **An IDT does not say what it claims.** An ACES IDT is one kind of object: camera-native to ACES2065-1, with the camera's white balanced to the ACES white thru a chromatic adaptation. Looks are meant to live in a separate stage, the LMT, but that separation is a convention, not a field, so a reader cannot check whether a given IDT is a characterization or carries a preference. There is also no tier: a transform measured on one unit and one derived from a model's published data are the same kind of object. Here every entry states its class and its [tier](tiers.md).
3. **The 1.x output rendering applied a fixed tone curve per channel.** The Reference Rendering Transform put every image thru a filmic tone scale applied to each channel in AP1. A per-channel curve changes the ratios between channels, so bright saturated colours shift in hue. Narrow-band light outside the working gamut, blue LEDs especially, rendered as magenta, and ACES 1.1 added an optional LMT to correct it. ACES 2.0 replaced the RRT and ODTs with a new output transform designed to preserve hue and compress gamut. In the terms used here, a rendering with a tone scale is a Creative op on the view side; it is recorded as one, and the scene data before it is left as measured.

In these terms an ACES IDT combines a characterization and a white balance in one matrix. This format records them as separate steps, so each can carry its own class.

## Pages

- [absolute.md](absolute.md) — the Absolute IDT: what it is, the strict field set, the read chain from counts to radiance, the two-camera test, and why ISO is derived rather than primary
- [relative.md](relative.md) — the Relative IDT: the DSR spectral solve, where its scale comes from, and what the lens has to do with it
- [layers.md](layers.md) — the K-channel compositing model: physical ops on K channels vs observer ops on the rendered view, blending space as a per-op parameter, the integer rules at the PSD/TIFF borders, and the op inventory (design, 2026-10-12)
- [creative.md](creative.md) — Creative layers: white balance as the naive DSR, camera looks, curves, and the rule for a second exposure scalar
- [technical.md](technical.md) — Technical ops: value-preserving mechanics, instrument remaps, and the line between them and Creative
- [tiers.md](tiers.md) — Unit, Model, Assumed: the trust axis, the tier that was added and removed, and why a tier says who measured and not whether they were honest
- [output.md](output.md) — the four classes on the way out: emissive, reflective and transmissive media, a medium's white and black as measurements (above 1 is real), which class each kind can keep, and what a renderer does about a medium's range
- [fields.md](fields.md) — every field a VSF image carries today, the wire names, the defaults absence implies, and the proposed additions with their status
