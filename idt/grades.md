# Grades

A grade says how a characterization was arrived at. It is the trust axis, independent of [class](README.md#class-and-grade-are-orthogonal), and it exists for one reason: so that a guess can never silently read as a measurement.

## The three

0. **`unit`** — measured on *this* physical unit. A chameleon magic-9 target scan thru this body and this lens. The strongest claim, and the only one that can say anything about the copy of the camera in your hand rather than the model.
1. **`model`** — the manufacturer's word for the model. A DNG `ColorMatrix1`/`ColorMatrix2`, shipped for every body of that type. Good, general, and blind to unit-to-unit variation and to the lens.
2. **`assumed`** — implied by a format's convention alone. An untagged JPEG read as sRGB; a WebP, a TIFF with no ICC profile; a JXL taken at its colour-encoding tag; a RAW with no matrix at all, taken as roughly VSF RGB. The convention might simply be wrong about the file, and the grade says so.

They are ordered. A profile lists entries best first, and a reader prefers a higher grade for the same illuminant. `unit` over `model` over `assumed`.

## What each permits

An `assumed` entry is honest about being a guess, which is its whole value: a viewer can show a JPEG in colour without pretending the JPEG told it anything, and the HUD reads `assumed` beside the source so nobody mistakes it for a measurement. A `model` entry can be trusted for the model's colour and not for this unit's. A `unit` entry can be trusted, full stop, for the glass and the body it names — and its provenance (`cal`, `patches`) is in the file so that trust can be audited rather than extended.

## The grade that was added and removed

On 2026-09-22 a fourth grade, `native`, was added for identity entries — samples that were already VSF RGB and needed no characterization — and reverted the same day.

The reasoning for adding it was correct as far as it went: an identity entry is not a guess, so `assumed` misdescribed it, and it is not a measurement, so `unit` misdescribed it too. Neither existing grade fit. The error was the conclusion. A grade answers "how much should I trust this matrix," and `unit > model > assumed` is an ordering of claim strength. "There is no matrix to trust" is not a point on that scale. It is a different kind of statement, and putting it in the enum that grades claim strength was a category error — the same shape as putting a look into a reference slot, made on the afternoon that mistake was being explained.

The format already had the representation: **no profile at all**. Untagged samples are VSF RGB by specification, so absence is the complete statement and the only honest one. See [README.md](README.md#absence-is-a-statement). The lesson generalises — when every value of an enum misdescribes a case, the case probably does not belong in that enum.

## Open and closed vocabularies

Two kinds of string live in a profile, and they fail differently on purpose.

`source` is **open**. It is a free-form derivation token — `magic9`, `dng_colormatrix1`, `assumed_srgb`, `no_colormatrix` — and an unknown token still names a working matrix. It just reads opaque. A reader never rejects a file over a source it has not seen.

`class`, `grade`, `layout` and `target` are **closed**. Their legal values are fixed, and a reader that meets one it does not know **fails loud** rather than degrading, per the rule that we control all clients. This is deliberate and it was exercised: a file written during the two hours `native` existed does not silently render wrong on a current reader. It fails to parse the profile, falls thru to the reader's fallback, and says why in the frame info.

## What a grade is not

It is not completeness. Whether an entry has spectral `curve`s behind it, or only a fitted matrix, is a separate axis — a `unit` entry with curves can be refit for any spectral set; a `unit` entry without them is bounded to its fit set. Both are `unit`. The curves' presence says which.
