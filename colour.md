# VSF RGB Colour Standard

## Executive Summary

**VSF RGB is a spectrally-defined colourspace using monochromatic primaries at 703nm, 523nm, and 462nm.** These wavelengths represent the peak responses of the AGB geometric mean model derived from human cone fundamentals. Unlike legacy standards specified by xy chromaticity coordinates, VSF RGB primaries are defined by physics and remain constant regardless of observer model updates.

## Why Spectral Definition Matters

Every colour standard you've used (sRGB, Adobe RGB, DCI-P3, Rec.709, Rec.2020) specifies its primaries using CIE xy chromaticity coordinates derived from the 1931 Standard Observer. This creates three fundamental problems:

### 0. Observer Model Is Frozen

The 1931 xy coordinates ARE the specification. You cannot update to better observer models (like CIE 2006) without changing the colourspace itself. The limitations and inaccuracies of 1931 data are permanently baked into these standards.

### 1. Specification Is Ambiguous

xy coordinates describe a perceptual response, not a physical stimulus. Multiple spectral power distributions can produce the same xy coordinate (metamerism). When a standard says "primary at xy (0.64, 0.33)", this doesn't uniquely specify which wavelengths to use.

### 2. The Observer Is Not Yours to Choose

A matrix between two xy-defined spaces has to be evaluated under the 1931 observer, because the primaries exist only as 1931 chromaticities. There is no better observer to switch to; the definition carries no spectrum to re-evaluate. Rec.2020 is the one standard that also publishes wavelengths (630nm, 532nm, 467nm), likely for hardware reasons. Wavelengths can be evaluated under any observer. Its xy coordinates were derived from those wavelengths under 1931, so the two agree under 1931 and disagree under anything newer. VSF treats the wavelengths as the specification.

## The Spectral Solution

**Primaries can be wavelengths. Conversions use the best available observer model.**

### The Three Primaries

VSF RGB uses three monochromatic primaries derived from the AGB geometric mean model of human cone perception:

- **Red (R)**: 703 nm
- **Green (G)**: 523 nm  
- **Blue (B)**: 462 nm

### Derivation From Cone Biology

The AGB model expresses colour perception as geometric mean ratios of cone responses:

- **A (Alpha)** = L/(L+M) - "redness"
- **G (Gamma)** = M/(L+S) - "greenness"
- **B (Beta)** = S/(S+M) - "blueness"

The VSF primaries are the wavelengths where each **ratio** achieves maximum value:
- 703nm: Peak of Alpha ratio (maximum L-cone dominance relative to M)
- 523nm: Peak of Gamma ratio (maximum M-cone dominance relative to L and S)
- 462nm: Peak of Beta ratio (maximum S-cone dominance relative to M)

These are not arbitrary choices or committee compromises - they are mathematical consequences of the geometric mean relationships in human cone spectral sensitivities. **These are not the peak sensitivities of individual cone types** - they are the wavelengths where each cone type achieves maximum relative dominance in its respective ratio.

## White Point: Illuminant E

**VSF RGB uses Illuminant E (equal energy spectrum), not D65.**

Illuminant E represents equal energy at all wavelengths. It is:
- Mathematically defined (E(λ) = constant)
- Independent of atmospheric conditions, geography, time of day, or weather

D65 represents "north sky daylight at sea level thru Earth's atmosphere at approximately 6504K correlated colour temperature." You cannot reproduce D65 in a laboratory without simulating planetary atmospheric conditions. This makes it fundamentally unsuitable as a reference standard for any system claiming physical reproducibility.

## Gamma: 2

**VSF RGB uses gamma 2 (pure square and square root operations).**

Not gamma 2.2. Not gamma 2.4. Not the sRGB piecewise abomination.

### Why Gamma 2?

Human vision is logarithmic response all the way to zero. Power functions are continuous to zero. Piecewise functions with linear segments near black add:
- Arbitrary thresholds (why 0.04045 in sRGB? why 0.081 in Rec.709?)
- Branching (slower, breaks vectorization)
- Complexity (more code, more bugs)
- Zero perceptual benefit

**Computational cost (modern x86-64):**
- Square (x²): ~3-5 cycles
- Square root (√x): ~13-18 cycles
- Power function (powf): ~150-520 cycles (highly implementation dependent)
- Piecewise function (sRGB/Rec.709): ~150-520 cycles + branch (1-20 cycles depending on prediction)

Gamma 2 encoding/decoding provides roughly **10-20× performance improvement** over gamma 2.2/2.4 power functions while remaining mathematically continuous to zero.

**Linearization:** `linear = encoded*encoded` (square)
**Encoding:** `encoded = linear^0.5` (square root)

### Quantization: ×256 Truncation (Not 255 and Rounding)

VSF uses **×256 truncation** for integer quantization, not the sRGB spec's ×255 rounding.

**Symmetric Bucket Sizes (But Asymmetric Round-Trips)**

×256 (VSF): Each integer represents exactly 1/256 of the range
- `0` represents [0., 1/256)
- `1` represents [1/256, 2/256)
- `128` represents [128/256, 129/256)
- `255` represents [255/256, 256/256) but clamps to 255
- All buckets are **exactly the same width**: 1/256

×255 (sRGB/Adobe RGB spec): Equal-width buckets, but asymmetric round-trips
- Encoding: `round(value * 255)` centers each bucket on the integer
- `0` represents [0., 0.5/255) — values [0, 0.00196)
- `1` represents [0.5/255, 1.5/255) — values [0.00196, 0.00588)
- `128` represents [127.5/255, 128.5/255) — values [0.4999, 0.5039)
- `255` represents [254.5/255, 1.] — values [0.9980, 1.0]
- All buckets **ARE the same width**: 1/255

**THE HORROR:** When decoding, they use `value / 255` with **NO +0.5**
- This returns the **LOWER EDGE** of the bucket, not the center!
- Encode 0.502 → `round(0.502 * 255)` = 128
- Decode 128 → `128 / 255` = 0.5019... ≠ 0.502
- You get back the bottom of the bucket [0.5, 0.504), not the center at 0.502

**Round-trip asymmetry:** Any value in bucket [0.5, 0.504) encodes to 128, but 128 decodes to 0.5019. The bucket center (0.502) does NOT round-trip to itself. Values slightly above center get pulled down on decode. Values slightly below center get pulled up less.

VSF's ×256 truncation has the SAME asymmetry (128 represents [0.5, 0.50390625) but decodes to 0.5), but at least it's honest about it: truncation is truncation. No pretending with `round()` that you're doing something symmetric.

**Performance difference:** 10-50× faster quantization, especially critical in video processing where you're quantizing millions of pixels per frame.

**Encoding:** `u8_value = (linear.sqrt() * 256.) as u8`
**Decoding:** `linear = (u8_value as f32 / 256.).powi(2)`

No branches. No rounding. No arbitrary division. Just bitshifts and truncation.

## HDR: Gamma 2, Then tanh (`gamma2tanh`)

**`gamma2tanh` is VSF RGB with headroom: gamma 2, then tanh, per channel.** Same primaries, same white, same ×2ⁿ truncation. Only the transfer differs, and only where it matters, near the top.

**Encoding:** `encoded = tanh(√linear)`
**Linearization:** `linear = atanh(encoded)²`

Linear 1.0 is diffuse white: paper white, the brightest thing in the scene that isn't a light source or a reflection of one. It encodes to tanh(1) = 0.7616, code 194 of 256. Everything brighter rolls off under the tanh instead of clipping, so speculars, the sun's glint and lamps keep their detail and their place above white.

### Why tanh, and why after the square root

- **Black stays gamma 2.** Near zero tanh(x) ≈ x, so the shadows are plain gamma 2 (within 1% for codes 1–16 at 8 bits) and black is 0.
- **The roll-off happens in eye space.** Gamma 2 is roughly perceptual lightness, so the tanh compresses where the eye measures brightness. The eye's own front end saturates the same way: a cone's response follows Naka–Rushton, R = I/(I+σ) = ½(1 + tanh(½·ln(I/σ))), a tanh of the natural log of intensity. The decode, atanh(v) = ½·ln((1+v)/(1−v)), is a natural log too.
- **The order doubles the headroom.** Decoding squares atanh, so the tanh's tail headroom squares on the way back to light and the stops double. tanh before the square root (√tanh(L)) gives about 60% of the stops for the same diffuse white code.
- **One curve.** No knee, no piecewise segments, no fitted constants, no branches.

### Headroom Comes From the Bit Depth

×2ⁿ truncation decodes at bucket bottoms, so the top code is (2ⁿ−1)/2ⁿ, never 1, and atanh stays finite. With ×255 normalisation the top code would be exactly 1, infinitely bright: VSF's quantization rule is what makes this colourspace well defined.

| Depth | Top code, above diffuse white |
|---|---|
| 8 bits | 3.28 stops |
| 10 bits | 3.86 stops |
| 12 bits | 4.34 stops |
| 16 bits | 5.12 stops |

Each extra bit adds about 0.35 to the tanh's reach (atanh(1−ε) ≈ ½·ln(2/ε)), squared on the way out. At 8 bits the stops above white get 32, 19, 7 and 1.6 codes: three usable stops. 16 bits gives all five.

### Per Channel

Each channel rolls off on its own, like gamma 2 applies per channel. VSF RGB's primaries come from the cone fundamentals, and each cone class saturates on its own curve, so per-channel roll-off is what vision does: very bright saturated colours shift and wash toward white the way they do in film and in the eye. A hue-preserving roll-off, if a display wants one, is a rendering decision, not something baked into the data.

### Quantization

The same ×2ⁿ truncation as gamma 2: a value's code is the largest code whose decoded value is at or below it. Gamma 2 gets decode → encode exact for free because sqrt(x²) = x in IEEE arithmetic. tanh and atanh are not exact inverses, so a decoded value on a bucket bottom can come back a hair under it and truncate one code low, and re-saving an image would drift it. The reference encoder therefore estimates with tanh and settles against the decoder's own values: every code at every depth from 1 to 16 bits round-trips (`tests/gamma2tanh.rs`). S44's 16-bit fraction can't separate every pair of adjacent 16-bit codes, so its 16-bit round trip is within one code.

### Tagging and Fallback

A plane or view says `encoding: g2t` (`idt/image.md`). `gamma2tanh` is the same tag spelled long: readers accept either, and writers write `g2t`. A reader that knows the tag expands the top end into the display's headroom: it puts diffuse white at a comfortable brightness and drives highlights above it. On an OLED only the highlight pixels get brighter. A reader that shows the codes as plain gamma 2 gets a correctly exposed picture: exact shadows, diffuse white at code 194 (about 0.8 stops under full scale, like any normally exposed photo) and highlights softly rolled off rather than clipped. A VSF 0.10 reader refuses the plane by name, because `encoding` is a closed vocabulary; it never renders it wrong.

Display headroom is reference-white placement. A panel that peaks at 1000 nits with diffuse white at 175 nits has log₂(1000/175) ≈ 2.5 stops above white. When content reaches further than the display can, roll it off into what the display has with the same tanh.

### 8-bit Pictures With Headroom *(planned 2026-10-17)*

ferros is HDR native, so every reader in the stack supports `gamma2tanh` by default, and the stack's 8-bit pictures use it **when the content has light above diffuse white**: renders of raws and VSF captures, HDR sources, and their preview planes, preview tiers (`idt/image.md`) and flat `image` sections (thumbnails, attachment previews; `builders::compressed_image`). Content with nothing above white (UI captures, text, SDR imports, most avatars and icons) stays plain VSF RGB. Exports to foreign formats keep their own transfer (an sRGB JPEG is sRGB).

The tagging is the existing rule: **untagged is VSF RGB, gamma 2**; a picture tagged `g2t` is VSF RGB with the extension (same primaries, same white, same ×2ⁿ truncation, tanh on top). A writer adds the tag only when it uses the curve.

At 8 bits the cost is small and stated: shadows are gamma 2 to within 1% for codes 1–16, diffuse white sits at code 194, and codes 195–255 hold 3.28 stops above white (32, 19, 7 and 1.6 codes per stop). The range up to white gets 194 codes instead of 256, so steps there are about 1.32× coarser; the stack dithers every 8-bit output already, which keeps that from banding. That cost is why SDR content stays gamma 2: the tanh would spend 61 of its 256 codes on range it never uses. A reader that ignores the tag sees a correctly exposed picture with soft highlights (Tagging and Fallback, above).

What this needs:

- **The flat `image` section carries `encoding`** (done 2026-10-17: `builders::compressed_image_encoded`, `parse_compressed_image().colour_encoding`, `vsf::image::decode_for_display`), absent ⇒ `gamma2`, so every existing avatar and preview reads unchanged. Avatars may be `g2t` too: a reader from before the field ignores it and shows the codes as gamma 2, about 0.8 stops darker with soft highlights.
- **AV1 previews say `transfer_characteristics = 2` (unspecified)** in the sequence header's colour config, since CICP has no code for this curve; the VSF tag is the statement.
- **Readers map for the display** (Display Mapping, below).

### Prior Art: HLG

HLG (ITU-R BT.2100) has the same structure: a square root below a knee, a compressive curve above it, diffuse white near 75% of the signal. It has proven that structure in broadcast. `gamma2tanh` replaces HLG's knee, its log segment, its hard clip and its three fitted constants (a = 0.17883277, b = 0.28466892, c = 0.55991073) with one smooth curve whose headroom is exact from the bit depth. Diffuse white lands at 76% of the signal, essentially where HLG puts it.

## Display Mapping

Storage and display are separate stages, and stay separate. An encoding (`gamma2`, `g2t`, `linear`) says what scene light a code holds, diffuse white at 1.0, and never changes with the screen. Display mapping says how one particular display shows that light, and is never written back into a file. A display's own highlight curve may resemble `g2t`'s tanh; that is coincidence, not a shared definition.

`colour::display::display_light(l, headroom, content_peak)` is the one mapping the stack's viewers call as their last step, so no two disagree:

- **headroom** is the display's peak over diffuse white: 1 for an SDR monitor, the HDR/SDR ratio on an HDR panel.
- **content_peak** is the brightest light the content holds: the image's own brightest code decoded, so a picture with mild highlights is not compressed for headroom it never uses.
- When the content fits (`content_peak ≤ headroom`) the mapping is exactly linear, `l / headroom`: SDR content on any display, and HDR content on a display with room for it.
- Otherwise it is linear up to a knee at 0.75 of the display's peak (`DISPLAY_KNEE`) and an extended-Reinhard shoulder above it, slope-continuous at the knee, monotone, and reaching the display's peak exactly at `content_peak`. On an SDR monitor a `g2t` picture's diffuse white then sits near 0.88 of peak, with its highlights in the top eighth.

The result is display light as a fraction of peak, linear; the app applies its display's own transfer last (sRGB for a desktop window, the panel's measured curve on ferros). `display_lut_u8` builds the per-image 256-entry table, and `vsf::image::decode_for_display` applies it for the α + darkness buffers fluor and toka draw.

## Observer Model: Stockman & Sharpe 2000 10° Cone Fundamentals

VSF RGB uses the **Stockman & Sharpe (2000) 10° cone fundamentals** for everything spectral: placing its own primaries, placing any other wavelength-defined primary, chromatic adaptation, and sensor characterisation. Earlier versions used the 2° set.

### Why This Matters

The observer model establishes perceptual equivalence between colourspaces: "What mixture of 703/523/462nm primaries produces the same L/M/S cone response as this input colour?"

**Key advantage of spectral definition:** When better observer models are published, VSF RGB can adopt them immediately. The primaries (703nm, 523nm, 462nm) never change. Only the transformation matrices get recalculated with improved cone fundamentals, yielding better perceptual accuracy.

Legacy standards specified in xy coordinates cannot do this - their primaries ARE those 1931 xy values. Updating the observer model would change the colourspace itself.

## Converting Between Colourspaces

**VSF RGB is the connection space.** Every conversion is:

```
Source RGB → Linear → VSF RGB → Linear → Target RGB
```

There is no XYZ or LMS hub. Each supported space has one matrix into VSF RGB and one out, derived once at build time; converting between two other spaces composes them. VSF RGB can be the hub because its primaries are physical: it is the monitor you would build if you could, three spectral lines and equal-energy white.

How a space's entry matrix is derived depends on how the space defines itself.

**Wavelength-defined primaries (VSF RGB, Rec.2020):** evaluate each primary's L, M, S response under the SS2000 10° cone fundamentals, build the matrix in LMS, and solve into VSF RGB. 1931 never appears.

**xy-defined primaries (sRGB/Rec.709, Adobe RGB, DCI-P3, ...):** an xy pair is a 1931 chromaticity and nothing else. There is no spectrum to re-evaluate under a better observer, so these matrices are built the only way they can be: xy → 1931 XYZ, with VSF's primaries placed in 1931 XYZ by the same colour matching functions, so both sides of the matrix share one observer. This is the one place 1931 is unavoidable, and it is confined to the entry matrix of the legacy space.

**Chromatic adaptation:** VSF RGB's white is Illuminant E; every legacy standard's is D65. The entry matrix includes a von Kries adaptation in SS2000 10° LMS from the source white to E, so a neutral in the source is a neutral (R = G = B) in VSF RGB and back. Without it, D65 white lands off the R = G = B axis, and VSF's YCbCr pays for that in chroma on every frame.

ICC profiles hand over D50-relative XYZ (the profile connection space illuminant), so the ICC path enters thru its own D50 → E adapted matrix, `XYZ_D50_2VSF_RGB`, rather than the colorimetric `XYZ2VSF_RGB`.

All conversions operate in **floating-point linear light**. Integer inputs are linearized. Integer outputs are gamma-encoded.

### Example: Rec.2020 → VSF RGB

**Step 1: Rec.2020 RGB → LMS**
- Calculate how much each Rec.2020 primary (630nm, 532nm, 467nm) excites L, M, S cones using the SS2000 10° cone fundamentals
- Scale the primaries so Rec.2020 (1, 1, 1) produces D65 in LMS
- Apply to input colour

**Step 2: adapt D65 → E**
- Von Kries scaling in LMS: divide by the D65 cone response, multiply by the E cone response

**Step 3: LMS → VSF RGB**
- Calculate how much each VSF primary (703nm, 523nm, 462nm) excites L, M, S cones using the same cone fundamentals
- Invert to get LMS → VSF RGB
- Apply to get VSF RGB output

The three steps fold into one 3×3 matrix at build time.

### Why We Use Rec.2020's Wavelengths, Not Its xy Coordinates

Rec.2020 publishes both. Under the 1931 observer they agree, because the xy values were derived from the wavelengths under 1931. Under any newer observer they describe different colours. The wavelengths are a physical stimulus and can be re-evaluated as observer models improve; the xy coordinates are frozen to 1931 forever. We use the wavelengths.

## Laboratory Reproducibility

Any laboratory with a spectrometer can verify VSF RGB:

**To reproduce VSF primaries:**
0. Generate or isolate monochromatic light at 703nm, 523nm, 462nm
1. Verify wavelengths with spectrometer
2. Done

**To reproduce sRGB primaries:**
0. Find the latest version of CIE 1931 Standard Observer data
1. Decide how to handle known flaws in 1931 data  
2. Calculate XYZ tristimulus values for published xy coordinates
3. Hope your implementation matches everyone else's
4. Discover it doesn't
5. Argue about which implementation is "correct"

VSF RGB primaries are physics. You measure them. They don't change.

**To reproduce D65 white point:**
0. Build a model atmosphere at Earth sea level density
1. Add appropriate aerosols and water vapor
2. Position yourself at correct latitude during correct weather conditions
3. Measure north sky daylight at the right time of day
4. Realize this is insane and just use published tables
5. Which version of D65 tables? There are several.

## Why VSF RGB Exists

VSF RGB is not a "better sRGB" or a "wider gamut version" of existing standards. The correct colourspace was always the physical one; sRGB is the impurity built on top of it. VSF RGB is what remains once that is removed:

### Physical Specification
Primaries are wavelengths that can be verified with instruments in any laboratory. No perceptual experiments. No committee votes. No accumulated historical baggage.

### Future-Proof Architecture  
Observer model updates improve conversion accuracy without changing the colourspace. Legacy standards are permanently locked to 1931 data.

### Computational Simplicity
Gamma 2 encoding is square/square root. No piecewise functions. No arbitrary thresholds. Several CPU cycles instead of hundreds.

### Mathematical Correctness
Equal energy white point (Illuminant E) is mathematically defined and laboratory reproducible. D65 requires a planet with an atmosphere. Oh, and a very specific sun.

## Reference Implementation

The VSF Rust library provides:
- Conversion functions between VSF RGB and common colourspaces
- Chromatic adaptation (D65 ↔ Illuminant E, von Kries in LMS, folded into every legacy entry matrix)
- Gamma conversion (including proper handling of sRGB/Rec.709/Rec.2020 piecewise curves)
- Transformation matrices derived from Stockman & Sharpe 2000 10° cone fundamentals

## If You Want xy Coordinates

If someone demands "What are the xy chromaticity coordinates of VSF RGB primaries?", the correct answer is:

**"Which observer model do you want to use?"**

- CIE 1931 2°?
- CIE 1964 10°?  
- CIE 2006 2°?
- Stockman & Sharpe 2000?

Calculate them yourself using your preferred observer. Different observers produce different xy values. **The wavelengths are the specification.** Everything else is derived.

---

## TL;DR

VSF RGB uses monochromatic primaries at 703nm/523nm/462nm (derived from AGB geometric mean model of cone perception), Illuminant E white point, and gamma 2 encoding. Primaries are specified by wavelengths (physics), not xy coordinates (perception). VSF RGB is the connection space for all conversions. Spectral matrices use the Stockman & Sharpe 2000 10° cone fundamentals and can be recalculated as better observer models are published; xy-defined legacy spaces enter thru 1931 XYZ because that is the only observer their definition supports. This makes VSF RGB objectively reproducible in any laboratory and free of the ambiguities and frozen limitations of legacy colour standards.
---

## Input Device Transforms

VSF RGB is where every characterization lands. How a sensor's counts get there — and what class of claim the transform makes about the scene — is specified in [`idt/`](idt/README.md): the four kinds of IDT (Absolute, Relative, Creative, Technical), the tier axis that says how a characterization was arrived at, the rule that a file with no profile *is* VSF RGB, and the field set that lets two different cameras at the same T-stop agree with no adjustment.
