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

## Status

**Proposed, 2026-10-08.** No field exists yet for a medium; this page fixes what the fields have to carry before they are named. See [`fields.md`](fields.md#proposed-additions--2026-10-08) for the proposed `medium` section.
