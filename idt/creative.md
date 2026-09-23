# Creative Layers

<table>
<tr>
<td align="center"><img src="images/ca.webp" width="360" alt="Chromatic adaptation (white balance)"><br>Chromatic adaptation — white balance</td>
<td align="center"><img src="images/creative.webp" width="360" alt="Stock camera settings"><br>A camera maker's stock look</td>
</tr>
</table>

A Creative transform departs from the measurement on purpose. It is faithful to a preference. There is nothing wrong with that — most images anyone looks at have one — as long as the file says so, so that a preference can never be mistaken for a fact about the scene.

Creative ops live in the `view_transform` log, in order, after characterization. The image data is never touched. The log is interpretation metadata: a reader replays it, or doesn't, and either way the measurement underneath is intact.

## Chromatic adaptation, honestly labelled

White balance is a diagonal scale in camera RGB: three multipliers, one per channel, chosen so some reference lands neutral. It removes the overall cast. It also gets the colours of individual objects wrong, because a diagonal is not a spectral solve — it cannot know that two surfaces reflecting the same camera RGB under tungsten would reflect differently under daylight.

It is the naive version of what a [Relative IDT](relative.md) does properly. DSR divides the illuminant out spectrally; white balance scales three numbers and hopes. Nearly every camera ships white balance as its default rendering, which is why so few images anyone sees are faithful to the object. The format does not forbid it. It files it where it belongs.

## Camera looks

A maker's stock picture style — modified colour, modified contrast, a house shoulder — is a Creative IDT, and most cameras ship one as their default. The images are not the scene. They are the maker's taste applied to the scene, and for many uses that is exactly what is wanted. For colour reproduction or measurement it is not, and the reader must be able to tell which it is holding.

This is the case ACES cannot express: vendors ship their look *inside* the Input Transform, and the single undifferentiated IDT slot has no vocabulary to say so. Here it is an entry or an op with `class = creative`, and nothing else in the file will read it as characterization.

## Curves, and the rest

Any tone or colour shaping applied for a look: a curve, a contrast change, a skew matrix, a highlight rolloff. The reserved op names in `view_transform` are `curve`, `contrast`, `skew_matrix`, `white_balance` and `dr_curve`; `exposure` is defined. The vocabulary is open — a reader that meets an op it does not know **must surface it, never silently drop it** — but the reserved names are the ones a writer should use for the things they name.

`dr_curve` is opsin's HDR rolloff, `(3x − x³)/2` on the linear display domain — a soft shoulder that reaches display white tangentially instead of clipping into it. It is a Creative op and it is recorded as one: the JPEG export and the screen encode thru the same function, so what was seen is what was written, and the file says a curve was applied.

## The rule for a second scalar

The characterization carries exactly one exposure scalar, and it is [Technical](technical.md): the `sensitivity` field, which states what the counts *mean*. It is part of the measurement.

Any exposure applied on top of that — to taste, to make it viewable, to match a neighbour — is a second scalar, and a second scalar is **Creative**. Same number, different claim. The first says "this is how bright it was." The second says "this is how bright I would like it shown." A reader that clamps the second has lost a preference. A reader that clamps the first has destroyed data. The class is how it knows which.

This is the whole DNG `BaselineExposure` problem in one sentence: DNG has one slot for both claims, and a reader cannot tell a calibration from a look.

## The layer stack

The `view_transform` log *is* the layer stack. `space` says where the ops apply — v0 is `vsf_rgb_linear`, after the elected characterization, before any display encode — and `ops` are applied in the order written.

**Proposed, 2026-09-22:** Technical ops precede Creative ops in the log. A curve applied before the sensitivity scalar would corrupt the claim the scalar makes; ordering them keeps the measurement whole underneath every layer. A writer that emits them out of order is wrong; a reader that meets them out of order should say so.

## Status of opsin's exposure op

opsin writes its slider as an `exposure` op with `class = technical`, following the current vsf doc's definition of Technical as hue-free mechanics. Under this specification that op is a second scalar and is therefore Creative. The change is proposed, not made; it is one word in one place, and it waits on the [Technical](technical.md) definition being adopted.
