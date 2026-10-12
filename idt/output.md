# The Same Four Classes, On The Way Out

The [README](README.md) defines an IDT as the relation between the light a sensor captured and the values a file hands you. Run the same relation the other way — from the values to the light a medium gives back — and nothing about the taxonomy changes. A print, a slide, an OLED, an e-ink panel: each either reproduces the light, reproduces the object, departs from both on purpose, or makes no claim. The four classes are the four classes. What changes on the output side is which of them the medium can *physically* keep.

This page is about that: what a medium is to the format, what its white and black mean, and which class an output transform can honestly claim.

## A medium gives light back in one of three ways

| kind | the light is | 1.0 means | examples |
|---|---|---|---|
| **emissive** | made by the medium | a stated radiance, as for [Absolute](absolute.md#open-decisions) | OLED, LCD, a projector's image |
| **reflective** | the *viewer's* light, returned | a perfect diffuse reflector under the viewing illuminant | paper, e-ink, a painted wall |
| **transmissive** | a lamp's light, filtered | the open lamp, nothing in the path | slide film, a backlit transparency |

The kind is the first fact to record, because it decides everything below. An emissive medium owns its light. A reflective or transmissive one borrows it, and whatever the viewer's lamp does to the image is added after the file has had its say.

## White and black are measurements, not 1 and 0

Every medium has a white — the most it can give back — and a black — the least. On a capture they are `white[k]` and `black[k]` in counts. On a medium they are the same two fields in the output space, per primary, measured on the scale the kind defines.

And they are **not** 1 and 0. A plain paper white is around 0.85 of a perfect diffuser. A paper with optical brighteners can read **above 1**: it absorbs ultraviolet and re-emits it as visible, so more visible light leaves than arrived, and the whites look bluer than white. A glossy or metallic surface exceeds 1 at the specular angle. On the black side, a print on a thin stock, or a display with a leaking backlight, sits above 0. These are real values and the format keeps them as measured. **A white above 1 is never normalised away** — the whole point of recording it is that it is above 1.

Two things the measurement has to state, or the number means nothing:

0. **The illuminant.** A brightener only fluoresces under light that carries ultraviolet, so an OBA paper's white is one value under daylight and another under an LED with no UV. The medium's `illuminant` is an EXIF LightSource code, exactly as a profile entry's is. (On the capture side this is the problem chameleon's UV indicator exists to flag.)
1. **The geometry.** Gloss is a direction. A white measured at 45°/0° is a different number from one measured at the specular angle. The geometry is part of the measurement, the way the emitter is part of an Absolute scan's provenance.

A medium's white and black carry a [tier](tiers.md) like any other characterization: **`unit`** when measured on this sheet, this panel; **`model`** from the stock's datasheet; **`assumed`** when the renderer took 1 and 0 because nothing better was known. Same axis, same rule — a guess must never pass as a measurement.

## Which class a medium can keep

| medium | Absolute | Relative | notes |
|---|---|---|---|
| **emissive** (OLED) | **yes** — it can emit the scene's actual light, cast and all, within its range | yes | the only kind that can be Absolute on its own |
| **reflective** (paper) | only under a known, controlled viewing light — a proofing booth | **yes** — its natural class | the viewer's lamp is added after the file; an absolute print under an uncontrolled lamp *becomes* relative the moment it is lit |
| **reflective, tiny gamut** (e-ink) | no | partly | the gamut is so small that squeezing colour into it is unavoidable, which is a Creative shift by definition |
| **transmissive** (slide) | yes, if the lamp is specified | yes | as a *capture*, slide film is Creative — the stock's dyes and curves are chemistry, and Velvia is not pretending otherwise |

Paper is the case the taxonomy was least obviously built for, and it fits best. A [Relative IDT](relative.md) yields reflectance: what the object would return under Illuminant E. A print *is* a reflectance. Relative in, relative out takes the original surfaces to printed surfaces that react to the room's light the way the originals would — as far as the inks' spectra allow. Three or four inks match under one illuminant; a wider ink set would hold under more. An "absolute" print ends up equivalent to a relative one in practice, because once the viewer's own light hits it, whatever cast was printed in is simply added to.

## What a renderer does, and what class that is

The medium's white and black are *facts*. What a renderer does about them is an *op*, and the op has a class like any other:

- **Scale the data's 1.0 onto the medium's white luminance** — one neutral scalar, ratios preserved. This is a stated scale with no taste in it, the same rule as the [sensitivity scalar](absolute.md#sensitivity-is-the-load-bearing-field): **Technical**. It is what "relative" output means here.
- **Clip what the medium cannot reach** — above its white, below its black — and say so. That is an Absolute rendering being honest about its range.
- **Lift the blacks to meet the medium's black** (black point compensation), **compress a gamut into a smaller one**, or **adapt chromatically to the medium's white** — a bluish paper white taken as neutral. Each alters claimed values for viewability, and each is **Creative**. Recorded as such, in the `view_transform` log, like any other preference.

ICC's "relative colorimetric" intent bundles the first and the last of these into one step — a scale *and* a chromatic adaptation to media white. This format keeps them apart, because one is a scale and the other is a look, and the file should be able to say which it did.

## A medium's response: control values to spectra

A white and a black say where a medium's range ends. What happens in between is the medium's **response**: the function from its control values to the light it gives back. It is the same word as the camera's response in [`relative.md`](relative.md#the-representation--decided-2026-10-10), in the opposite direction. A camera's response maps a spectrum to counts; a medium's maps control values to a spectrum.

| kind | control values | the response gives |
|---|---|---|
| reflective | colorant coverage, 0 to full, per colorant (ink, toner, paint) | spectral reflectance factor, per band |
| transmissive | colorant density or coverage (film dyes, ink on film) | spectral transmittance, per band |
| emissive | drive level per primary | spectral radiance, per band |

### Why a table and not a formula

The response is a property of the combination, not of the colorants: the same mix of the same inks reads differently on a different paper, a different press, a different screen ruling, a different day. Light scatters inside the paper and makes dots look larger than they are (optical dot gain); one ink sits on another imperfectly (trapping); pigments both absorb and scatter. So the truth is **measured**: print a chart of coverage combinations (IT8.7/4 is 1617 patches in the four-ink cube), read each with a spectrophotometer, and the response is that sampled function, interpolated.

Parametric models predict it from much less: the spectral Neugebauer model with the Yule–Nielsen correction needs the spectra of the 2^K solid overprints and one exponent; the cellular form adds intermediate primaries; Kubelka–Munk works from absorption and scattering per colorant. They reach about 1 to 2 ΔE on a well-behaved press and miss where trapping and paper absorption misbehave. They are also the only option once K is large: a 9-node grid over eight inks is 43 million spectra, a cellular model of the same printer about 6,500 measured patches. So a response is stored as **a sampled table when one was measured, a model when it was predicted**, and says which.

An emissive response has the same shape. A display with no crosstalk between primaries is a 3×3 matrix and three tone curves; an OLED whose primaries interact is not, and the table holds it.

### Fluorescence

A fluorescent medium's light at one wavelength depends on the illuminant at every shorter wavelength, so one spectrum per node cannot describe it. The complete description is the **Donaldson matrix** `B(λ_in, λ_out)`: output = `B` × illuminant. Reflectance is its diagonal; an optical brightener is a block below it, absorbing 350 to 400 nm and emitting 420 to 470 nm. This is where a white above 1 comes from (above, "White and black are measurements").

Full Donaldson matrices are measured with a bispectrometer, which production does not have. Production measures under two conditions: **M2**, ultraviolet cut, which is pure reflectance; and **M1**, D50 including its ultraviolet, which adds the brightener's emission under D50. Their difference is the emission at that one excitation. For a single fluorophore, which an optical brightener is, the usable approximation is **separable**:

```
B(λ_in, λ_out)  ≈  diag(R(λ)) + E(λ_out) · X(λ_in)ᵀ
```

`R` is the M2 reflectance at a node, `E` its emission spectrum, `X` one excitation curve for the substrate. Under a new illuminant the emission scales with that light's power across `X` relative to the measurement's. Ink over the paper attenuates both the exciting ultraviolet and the emitted blue, so `E` falls with coverage and is stored per node like `R`. The approximation is wrong for colorants that fluoresce themselves (some magentas and spot inks do), and a chart cannot reveal that without an M1/M2 pair per patch. The format allows the full matrix per node for whoever has one; nobody is obliged to fill it.

### Rendering a response

Flattening happens at render time, for the view's illuminant and observer, never in the file. Integrating every node's spectrum against an N-channel observer under an illuminant gives a node table of N values per node; the per-pixel work is then a K-dimensional interpolation into it, the same operation an ICC profile's `A2B` table performs. Integration is linear, so interpolating precomputed node colours equals integrating interpolated spectra: precomputing per view costs nothing in accuracy, and for a 9⁴ grid it is microseconds, cached per (illuminant, observer).

- **Observer**: by default Stockman and Sharpe 2000 10°, on which VSF RGB is defined ([`colour.md`](../colour.md)); the node table lands in VSF RGB and the display path takes it from there. A 10° field also suits a print viewed at size. CIE 1931 2° is another observer, used when a ΔE has to compare with published numbers.
- **Illuminant**: the light the print will be seen under, chosen by [`relight`](layers.md), not stored in the medium. ISO 3664 D50 is the proofing default; a print that will hang under tungsten is proofed under tungsten. The medium records the condition it was *measured* under (M1, M2), which only matters for scaling the emission.

### Ink planes

An ink plane is device data, not light. It has K channels named by colorant, values are coverage with `black` = 0 and `white` = full coverage, and its appearance exists only through a medium's response. It never enters a blend with a light plane. It is the product of a separation, or a CMYK import kept byte for byte. Going the other way, from a rendered view to ink amounts, is an inversion with no unique answer: total ink limit, how much black replaces grey (GCR), gamut compression. Every one of those is a choice, so **separation is a Creative op with its parameters recorded**, and it is the last thing to build.

### What an ICC output profile carries, mapped

An ICC CMYK profile (U.S. Web Coated SWOP, Coated FOGRA39, and the rest) is a description, a copyright, a media white point and six lookup tables. It holds no spectra, no ink densities, no dot gain curves, no ink limit:

| ICC tag | what it is | in VSF |
|---|---|---|
| `A2B0/1/2` | colorant amounts → Lab, D50, CIE 1931 2°, one table per rendering intent, 9⁴ to 11⁴ nodes | the response **at one illuminant and one observer, spectra discarded**: imported as a colorimetric response at tier `model`. It renders, but cannot be relit and cannot change observer |
| `B2A0/1/2` | Lab → colorant amounts | a **separation**, with the ink limit, black generation and gamut mapping baked in and unrecoverable: kept verbatim, used only to separate, marked Creative |
| `wtpt` | media white, D50 | the medium's `white` (as XYZ D50, so its tier is `model`) |
| `bkpt` (when present) | media black | the medium's `black`; otherwise the darkest node of `A2B`, which is also the only trace of Dmax |
| `gamt` | gamut boundary | kept verbatim; derivable from the response |
| dot gain, ink limit, GCR | not stored | implicit in the tables; a reader can observe the ink limit as no `B2A` node exceeding it, nothing more |

The spectra a profile was built from live in its **characterization dataset**: a CGATS text file of chart coverages and measurements, published by the standards bodies. Some include spectral reflectance columns (FOGRA51 and FOGRA52 do, 380 to 730 nm at 10 nm); others are colorimetric only; check before relying on one, and check its licence before redistributing it. A dataset with spectra imports as a sampled spectral response at tier `model`; a chart printed on your own press and read on your own instrument is tier `unit`. The chameleon target cannot produce one: it recovers a camera's response, not a medium's.

## Status

**Proposed, 2026-10-08; the response added 2026-10-12.** No field exists yet for a medium; this page fixes what the fields have to carry before they are named. See [`fields.md`](fields.md#proposed-additions--2026-10-08) for the proposed `medium` section and [`fields.md`](fields.md#proposed-additions--2026-10-12-the-medium-response) for the response fields.
