# Tiers

A tier says how a characterization was arrived at. It is the trust axis, independent of [class](README.md#class-and-tier-are-orthogonal), and it exists for one reason: so that a guess can never silently read as a measurement.

## The three

0. **`unit`** — measured on *this* physical unit. A chameleon magic-9 target scan thru this body and this lens. The strongest claim, and the only one that can say anything about the copy of the camera in your hand rather than the model.
1. **`model`** — a characterization of the *model*, not of this unit. The manufacturer's DNG `ColorMatrix1`/`ColorMatrix2`, shipped for every body of that type — or a third party's measurement of the model, which is what every raw converter carries for the proprietary formats that ship no matrix at all (Adobe's, as dcraw, LibRaw and rawler copied them). General, and blind to unit-to-unit variation and to the lens. Whose it is lives in `source`, and that matters — see [below](#a-tier-is-who-measured-not-whether-they-were-honest).
2. **`assumed`** — implied by a format's convention alone. An untagged JPEG read as sRGB; a WebP, a TIFF with no ICC profile; a JXL taken at its colour-encoding tag; a RAW with no matrix at all, taken as roughly VSF RGB. The convention might simply be wrong about the file, and the tier says so.

They are ordered. A profile lists entries best first, and a reader prefers a higher tier for the same illuminant. `unit` over `model` over `assumed`.

## What each permits

An `assumed` entry is honest about being a guess, which is its whole value: a viewer can show a JPEG in colour without pretending the JPEG told it anything, and the HUD reads `assumed` beside the source so nobody mistakes it for a measurement. A `model` entry can be trusted for the model's colour and not for this unit's. A `unit` entry can be trusted, full stop, for the glass and the body it names — and its provenance (`cal`, `patches`) is in the file so that trust can be audited rather than extended.

## A tier is who measured, not whether they were honest

A tier is a claim about **provenance**: this unit, this model, or nobody. It says nothing about whether the numbers in the slot are a measurement. Class is the axis for that, and class is only as truthful as the writer.

This stopped being theoretical on 2026-10-08. The Sigma fp writes its colour matrices into the DNG itself — the one case among the cameras to hand where the maker, not a third party, fills the Model slot — and it writes a **different pair per colour mode**, same sensor, same firmware. Scored on five frames of a VERICHROME target against the target's own measured spectra (mean ΔE2000, white balance from the target's greys, so the matrix's colour error alone): mode OFF, the "flat" look, renders a measured surface at 0.57× its true chroma, 8.2 ΔE; Standard/Neutral at 0.80×, 5.0 ΔE; the Adobe-derived matrix RawTherapee carries for the same body at 0.92×, 4.0 ΔE; the best any 3×3 could do on those frames, 2.5. Adobe's matrices for two Sony bodies, scored the same way on a ColorChecker, sat 0.5–1 ΔE above that floor at 0.89–1.0× chroma — an honest measurement. Sigma's is a look, in the slot the DNG specification reserves for the characterization, and every DNG reader on earth trusts it as one.

The tier axis cannot see this and should not try to; it would be the `native` mistake again, a judgement smuggled into an enum that claims something else. What the format *does* have is `source`, open and free-form, which says whose per-model characterization an entry is. A reader is free to rank sources within a tier, and opsin does: a chameleon entry (`unit`) over Adobe's per-model measurement (`model`, source `adobe_model_matrix`) over the file's own (`model`, source `dng_colormatrix1`). The ordering is the reader's policy, stated in the reader. The file just says who.

The lesson for a writer is the same one as everywhere else in this directory: if the numbers were chosen for how they look, say `creative`, and the slot stays honest.

## The tier that was added and removed

On 2026-09-22 a fourth tier, `native`, was added for identity entries — samples that were already VSF RGB and needed no characterization — and reverted the same day.

The reasoning for adding it was correct as far as it went: an identity entry is not a guess, so `assumed` misdescribed it, and it is not a measurement, so `unit` misdescribed it too. Neither existing tier fit. The error was the conclusion. A tier answers "how much should I trust this matrix," and `unit > model > assumed` is an ordering of claim strength. "There is no matrix to trust" is not a point on that scale. It is a different kind of statement, and putting it in the enum that tiers claim strength was a category error — the same shape as putting a look into a reference slot, made on the afternoon that mistake was being explained.

The format already had the representation: **no profile at all**. Untagged samples are VSF RGB by specification, so absence is the complete statement and the only honest one. See [README.md](README.md#absence-is-a-statement). The lesson generalises — when every value of an enum misdescribes a case, the case probably does not belong in that enum.

## Open and closed vocabularies

Two kinds of string live in a profile, and they fail differently on purpose.

`source` is **open**. It is a free-form derivation token — `magic9`, `dng_colormatrix1`, `assumed_srgb`, `no_colormatrix` — and an unknown token still names a working matrix. It just reads opaque. A reader never rejects a file over a source it has not seen.

`class`, `tier`, `layout` and `target` are **closed**. Their legal values are fixed, and a reader that meets one it does not know **fails loud** rather than degrading, per the rule that we control all clients. This is deliberate and it was exercised: a file written during the two hours `native` existed does not silently render wrong on a current reader. It fails to parse the profile, falls thru to the reader's fallback, and says why in the frame info.

## Why "tier" and not "grade"

This axis was called `grade` until 2026-09-22. In this format's own industry, "grade" is the creative colour pass — the thing a colourist does in Resolve. So `ProfileGrade` read as "the profile's look," which is the precise opposite of what it is: the trust level of a *measurement*. A taxonomy whose whole job is keeping Creative from being mistaken for characterization had named the characterization's trust axis with the industry's word for Creative.

"Tier" was already the docs' word for the concept — "tiered characterization," "best entry first" — and it is ordinal, which the axis is. The values `unit`, `model`, `assumed` were never the problem and did not change. The wire field is `tiers`. No file was ever written with the old name, so there is no alias: a reader that meets `grades` fails loud, as it does for any unknown field.

## What a tier is not

It is not completeness. Whether an entry has spectral `curve`s behind it, or only a fitted matrix, is a separate axis — a `unit` entry with curves can be refit for any spectral set; a `unit` entry without them is bounded to its fit set. Both are `unit`. The curves' presence says which.
