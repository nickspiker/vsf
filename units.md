**FUNDAMENTAL UNITS**

---

## Base Units (2 Fundamental)

### 0. TIME - COMPLETE DERIVATION WITH SOURCES

### Definition
**One complete oscillation period of the electromagnetic radiation emitted during the hydrogen-1 hyperfine transition between F=0 and F=1 ground states, as measured at the barycentric reference frame of the Milky Way-Andromeda galaxy system.**

### Barycentric Frequency Calculation

**Starting Point:** Hydrogen 21cm line measured on Earth's surface
- **f_Earth = 1,420,405,751.768 Hz** (NIST/CODATA standard reference)

---

### Gravitational Potential Components

Earth's surface gravitational potential relative to the barycenter (Φ = 0):

#### 0. Earth's Gravitational Potential

**Formula:** Φ_E = -GM_E/r_E

**Constants:**
- G = 6.67430(15)×10⁻¹¹ m³/(kg·s²) [CODATA 2018]
- M_E = 5.97219×10²⁴ kg [IAU 2015]
- r_E = 6.3781×10⁶ m (mean Earth radius) [IUGG]

**Calculation:**
Φ_E = -(6.67430×10⁻¹¹ × 5.97219×10²⁴)/(6.3781×10⁶)
**Φ_E = -62.53 MJ/kg**

**Uncertainty:** ±0.1% (dominated by G measurement uncertainty)

---

#### 1. Sun's Gravitational Potential at Earth's Orbit

**Formula:** Φ_S = -GM_S/r_ES

**Constants:**
- M_S = 1.98847×10³⁰ kg [IAU 2015]
- r_ES = 1.49598×10¹¹ m (1 AU) [IAU 2012]

**Calculation:**
Φ_S = -(6.67430×10⁻¹¹ × 1.98847×10³⁰)/(1.49598×10¹¹)
**Φ_S = -887.4 MJ/kg**

**Uncertainty:** ±0.01%

---

#### 2. Milky Way Gravitational Potential at Solar Position

**Method:** Derived from escape velocity measurements via Φ = -v_esc²/2

**Source:** Gaia satellite stellar velocity measurements (2018-2025)

**Measured Escape Velocities at Solar Position:**
- Monari et al. (2018, Gaia DR2): v_esc = 580 ± 63 km/s
- Koppelman et al. (2020, Gaia DR2): v_esc = 550 ± 30 km/s (corrected)
- Necib & Lin (2021, Gaia eDR3): v_esc = 485 ± 15 km/s
- Wu et al. (2025, Gaia DR3): v_esc = 524 ± 13 km/s

**Adopted Value (conservative middle estimate):**
**v_esc = 530 ± 50 km/s**

**Calculation:**
Φ_MW = -(530,000 m/s)² / 2
**Φ_MW = -140.5 GJ/kg**

**Range:** -120 to -168 GJ/kg (based on measurement uncertainty)

**Notes:** 
- Measurements use high-velocity halo stars from Gaia astrometry
- Escape velocity defined as minimum speed to reach 3× virial radius
- Includes contributions from visible matter, dark matter halo, and orbital kinetic energy
- Does NOT include Andromeda contribution (negligible at 2.5 Mly distance)

---

#### 3. Total Gravitational Potential at Earth Surface

**Φ_total = Φ_E + Φ_S + Φ_MW**

Φ_total = -0.06253 - 0.8874 - 140.5 GJ/kg
**Φ_total = -141.5 GJ/kg**

**Uncertainty Range:** -131 to -169 GJ/kg (dominated by Milky Way measurement)

---

### Gravitational Time Dilation Correction

**Formula:** Δf/f = ΔΦ/c²

**Where:**
- ΔΦ = Φ_barycenter - Φ_Earth = 0 - (-141.5 GJ/kg) = 141.5 GJ/kg
- c = 299,792,458 m/s (exact, defined)
- c² = 8.98755178736×10¹⁶ m²/s²

**Calculation:**

Δf/f = (141.5×10⁹ J/kg) / (8.98755178736×10¹⁶ m²/s²)

**Δf/f = 1.575×10⁻⁶**

**Uncertainty Range:** 1.46×10⁻⁶ to 1.88×10⁻⁶

---

### Barycentric Hydrogen Frequency

**f_barycentric = f_Earth × (1 + Δf/f)**

f_barycentric = 1,420,405,751.768 Hz × (1 + 1.575×10⁻⁶)

**f_barycentric = 1,420,408,188 Hz**

**Uncertainty Range:** 1,420,407,826 to 1,420,408,420 Hz

**Difference from Earth measurement:** +2,436 Hz (±300 Hz)

---

### Base Time Unit Period

**τ₀ = 1 / f_barycentric**

τ₀ = 1 / 1,420,408,188 Hz

**τ₀ = 7.04043×10⁻¹⁰ seconds**

**Uncertainty:** ±0.0002×10⁻¹⁰ seconds

---

### Verification Procedure

**Anyone can verify this calculation by:**

0. **Measure local hydrogen 21cm line frequency** using radio receiver tuned to ~1.42 GHz
1. **Calculate local gravitational potential:**
   - Earth: Use local latitude/elevation
   - Sun: Use current Earth-Sun distance (varies ±3% annually)
   - Galaxy: Use current Gaia data releases for escape velocity
2. **Apply correction:** f_barycentric = f_local × (1 + Φ_local/c²)
3. **Compare to specified value**

**Expected agreement:** Within ±300 Hz given measurement uncertainties

---

### Future Updates

**This specification uses:**
- CODATA 2018 physical constants
- IAU 2015 astronomical constants
- Gaia DR2/DR3 (2018-2025) galactic measurements

**Future revisions may be warranted if:**
- Gaia mission continues and refines escape velocity measurements (expected ±5% improvement)
- Better dark matter halo models become available
- Measurement of barycenter potential becomes feasible

**Revision policy:** Any change to base frequency must:
1. Cite new observational data
2. Show calculation with uncertainties
3. Be approved by community consensus
4. Maintain backward compatibility via documented conversion factors

**Symbol:** ?

---

### 1. COUNT  
**Definition:** Dimensionless integer representing discrete quantities.

**Rationale:**
- Fundamental to quantum mechanics (everything is countable)
- Exact by definition (no measurement error)
- Basis for mass, charge, particle number

**Symbol:** ?

---

## Derived Units

### LENGTH
**Definition:** Distance light travels in vacuum during one time unit.

**Value:** ~0.2108 meters (~21.1 cm)

**Symbol:** ?

---

### MASS
**Definition:** Count of proton masses.

**Derivation:** M = N × m_p where m_p = proton rest mass

**Symbol:** ?

---

### CHARGE
**Definition:** Count of elementary charges.

**Derivation:** Q = N × e where e = 1.602176634×10⁻¹⁹ C

**Symbol:** ?

---

### TEMPERATURE
**Definition:** Energy per particle per degree of freedom.

**Derivation:** T = E/(N × k_B) where k_B = Boltzmann constant

**Alternatively:** Express directly in energy units (like electron-volts)

**Symbol:** ?