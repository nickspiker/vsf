# VSF RGB Colour Standard

## Executive Summary

**VSF RGB is a spectrally-defined colour space using monochromatic primaries at 703nm, 523nm, and 462nm.** These wavelengths represent the peak responses of the AGB geometric mean model derived from human cone fundamentals. Unlike legacy standards specified by xy chromaticity coordinates, VSF RGB primaries are defined by physics and remain constant regardless of observer model updates.

## Why Spectral Definition Matters

Every colour standard you've used (sRGB, Adobe RGB, DCI-P3, Rec.709, Rec.2020) specifies its primaries using CIE xy chromaticity coordinates derived from the 1931 Standard Observer. This creates three fundamental problems:

### 0. Observer Model Is Frozen

The 1931 xy coordinates ARE the specification. You cannot update to better observer models (like CIE 2006) without changing the colour space itself. The limitations and inaccuracies of 1931 data are permanently baked into these standards.

### 1. Specification Is Ambiguous

xy coordinates describe a perceptual response, not a physical stimulus. Multiple spectral power distributions can produce the same xy coordinate (metamerism). When a standard says "primary at xy (0.64, 0.33)", this doesn't uniquely specify which wavelengths to use.

### 2. Accumulated Transformation Errors

Converting between colour spaces requires chaining transformations thru XYZ tristimulus space, each step accumulating floating-point error and observer model inconsistencies. Rec.2020 kind of solves this by listing xy coordinates and wavelengths (630nm, 532nm, 467nm) - likely specified for hardware implementation reasons. However, their published xy coordinates don't match those wavelengths using the 2006 standard observer. Now you have two specifications that define different colours. Which one is "correct"? Nobody knows. VSF uses their wavelengths and ignores the xy coordinates.

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

**Linearization:** `linear = encoded^0.5` (square root)  
**Encoding:** `encoded = linear*linear` (square)

That's it. No branches. No arbitrary constants. No special cases. Stupidly fast.

## Observer Model: CIE 2006 2° Standard Observer

VSF RGB currently uses the **CIE 2006 2° Standard Observer** (cone fundamentals) for all colour space conversions.

### Why This Matters

The observer model is used to establish perceptual equivalence between colour spaces: "What mixture of 703/523/462nm primaries produces the same L/M/S cone response as this input colour?"

**Key advantage of spectral definition:** When better observer models are published, VSF RGB can adopt them immediately. The primaries (703nm, 523nm, 462nm) never change. Only the transformation matrices get recalculated with improved cone fundamentals, yielding better perceptual accuracy.

Legacy standards specified in xy coordinates cannot do this - their primaries ARE those 1931 xy values. Updating the observer model would change the colour space itself.

## Converting Between Colour Spaces

All conversions go thru LMS cone space using the most recent observer model:

```
Source RGB → Linear → LMS → Linear → Target RGB
```

### Example: Rec.2020 → VSF RGB

**Step 1: Rec.2020 RGB → LMS**
- Calculate how much each Rec.2020 primary (630nm, 532nm, 467nm) excites L, M, S cones using CIE 2006 cone fundamentals
- Build transformation matrix from Rec.2020 primaries to LMS
- Apply to input colour

**Step 2: LMS → VSF RGB**  
- Calculate how much each VSF primary (703nm, 523nm, 462nm) excites L, M, S cones using CIE 2006 cone fundamentals
- Build transformation matrix from LMS to VSF primaries
- Apply to get VSF RGB output

This approach:
- Uses wavelengths directly (no xy coordinate ambiguity)
- Applies a single observer model consistently
- Produces reproducible transformation matrices
- Avoids accumulated errors from chaining thru 1931 tristimulus space

### Why We Ignore Rec.2020's xy Coordinates

Rec.2020 publishes both wavelengths (630/532/467nm) and xy chromaticity coordinates for its primaries. **These specify different colours.** When you calculate xy coordinates from their published wavelengths using the 2006 Standard Observer, the results don't match their published xy values.

We use their wavelengths and ignore their xy coordinates. Wavelengths are unambiguous. xy coordinates lie.

### Converting to Legacy xy-Coordinate Standards

For standards specified only in xy coordinates (sRGB, Adobe RGB, DCI-P3, etc.), VSF conversions:
- Go thru LMS space to avoid xy coordinate inconsistencies
- Use published transformation matrices when no spectral definition exists
- Apply the current observer model (CIE 2006 2°) consistently
- Handle gamma conversion properly (including those baroque piecewise curves)

All conversions operate in **floating-point linear light**. Integer inputs are linearized. Integer outputs are gamma-encoded.

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

VSF RGB is not a "better sRGB" or a "wider gamut version" of existing standards. It is colour done correctly from first principles:

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
- Illuminant scaling (D65 ↔ Illuminant E when converting to/from legacy standards)
- Gamma conversion (including proper handling of sRGB/Rec.709/Rec.2020 piecewise curves)
- Transformation matrices derived from CIE 2006 2° cone fundamentals

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

VSF RGB uses monochromatic primaries at 703nm/523nm/462nm (derived from AGB geometric mean model of cone perception), Illuminant E white point, and gamma 2 encoding. Primaries are specified by wavelengths (physics), not xy coordinates (perception). Conversions use CIE 2006 2° cone fundamentals and can be updated as better observer models are published. This makes VSF RGB objectively reproducible in any laboratory and eliminates the accumulated errors, ambiguities, and frozen limitations of legacy colour standards.