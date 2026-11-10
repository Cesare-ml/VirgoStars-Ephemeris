# MHB2000 simplified lunar arguments for planetary nutation

**2026-11-11 · Original Rust mathematical primitive, not full IAU 2000A nutation.**

## Why an additional lunar-argument model is necessary

The original IAU 2000A MHB2000 implementation calculates the 678 lunar-solar
and 687 planetary terms using **different lunar argument conventions**.
[IERS Conventions (2010), §5.7.3, after Eq. (5.44)](
https://iers-conventions.obspm.fr/content/tn36.pdf) explicitly documents the
use of **simplified Delaunay variables** for planetary nutation, and notes that
the effect of substituting the standard full variables is small but nonzero
at microarcsecond scale.

The full five-argument polynomial model is being separately developed in
[draft PR #52](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/52).
**It must not silently be reused for the planetary 687 terms.**

### Scientific expressions

The original MHB2000 planetary family uses **four** simplified lunar
arguments, in radians, with T the numerical Julian centuries from J2000:

```text
l      = 2.355555980 + 8328.6914269554 × T
F      = 1.627905234 + 8433.4661581310 × T
D      = 5.198466741 + 7771.3771468121 × T
Omega  = 2.182439200 -   33.7570450000 × T
```

These historical coefficients and their distinct planetary use are
described in the IAU SOFA/ERFA MHB2000 nutation documentation; they can
be cross-checked independently with the historical implementation records
and the [IERS chapter 5 scientific explanation](
https://iers-conventions.obspm.fr/content/chapter5/icc5.pdf).
Only mathematical expressions/numerical values are used here; the Rust
evaluation was independently authored, without translating external
program/source code or incorporating a periodic coefficient table.

**The mean solar anomaly l' is absent in the historical planetary-term
calculation.** Therefore the Rust API models four arguments, ordered
`[l,F,D,Omega]`, not a misleading five-argument vector with a silently
manufactured solar term. Any future input that has a nonzero multiplier
for the l' slot is rejected by the combined-phase helper.

### Time and units

`mhb2000_planetary_lunar_arguments(JulianDate)` requires a split
Julian date explicitly labeled **TDB**, matching the mathematical
argument/rate interfaces of draft PRs #52, #53 and #54. The polynomial
parameter is

```text
T = (JD_TDB - 2451545.0) / 36525
```

Returned `angles_rad` are normalized to `[0, 2π)`; returned rates
are derivatives of the unwrapped expressions in **rad/TDB day**.
No UTC/TAI/TT/UT1 conversion occurs. **Important scientific limitation:**
the canonical IAU/SOFA `Nut00a` interface uses **TT** input, and its
full precession/nutation derivatives have a TT convention. This module
does not claim to resolve that TT/TDB distinction, and MUST NOT be
substituted directly into the eventual TT-frame pipeline without an
explicitly validated time argument conversion/approximation.

J2000 ±2 TDB Julian centuries is a conservative **engineering test
window**, not an official precision or validity bound.

## Safe phase composition with the separate nine planetary arguments

The new `MhbPlanetaryLunarArguments::phase([i8;4])` creates a normalized
four-lunar-variable phase and rate.

`phase_with_planets([i8;14], planetary_angles_rad:[f64;9],
planetary_rates_rad_per_tdb_day:[f64;9])` accepts the canonical
**14-slot** order required by future tables:

```text
[l,l',F,D,Omega,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,p_A]
```

It rejects `multipliers[1] != 0` as
`UnsupportedSolarAnomalyMultiplier`; it never inserts a fake value for
l'. The supplied last-nine arrays must contain finite, normalized
angles (radians) and finite signed rates (rad/TDB day), in F6–F14
order. They are **not** calculated here. It is the integration layer's
responsibility to ensure all input arguments refer to the same epoch
and to use the **historical Neptune convention** exposed in
[draft PR #53](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/53)
when appropriate.

This function **only** constructs a harmonic phase and its velocity.
It does not calculate Δψ, Δε or their derivatives. The compensation
kernel for later coefficient accumulation remains independently
proposed in [draft PR #54](
https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/54).

## Ten test cases and independent-check boundaries

`crates/ephemeris-core/tests/mhb_planetary_lunar.rs` provides **10**
deterministic tests:

1. J2000 published constants, signed rates, no solar slot.
2. Independent high-precision (70-digit arithmetic) **decimal**
   calculations of all four reduced phases at numeric `T=0.8`.
3. Explicit non-equality with the full IERS Delaunay lunar anomaly,
   using a published ERFA standalone `Fal03` reference value.
4. Centered-angle finite differences against analytical day rates across
   five dates.
5. Four-argument phase/rate and sign handling.
6. A 14-slot harmonic phase composed from four lunar and nine
   externally supplied planetary inputs, with independent expected sum.
7. Mandatory rejection of nonzero solar-anomaly multipliers.
8. Nonfinite, negative and nonnormalized external planetary phases/rates.
9. Phase wrapping near the 2π boundary without false velocity jumps.
10. Explicit TDB input, unsafe-JD and supported-date-boundary guards.

The decimal numerical reference tests verify **the four simplified
mathematical polynomials**, not independent actual IAU 2000A nutation
angles or real NASA/JPL planetary vectors. No official 1,365-term
coefficient table is included. No claimed Swiss Ephemeris parity.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test mhb_planetary_lunar
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Remaining scientific/integration gates

The full IAU 2000A implementation still requires rights-verified
official 678+687 harmonic coefficient tables, audited table-column
mapping, validation of the exact series term conventions, a TT/TDB
time-rate contract compatible with the precession and nutation matrices,
and pinned independent SOFA/ERFA numerical checks at multiple epochs.
This PR can compile independently of all those still-draft stages.

## CPAL-1.0 modification record (2026-11-11)

Added original `src/mhb_planetary_lunar.rs`, associated ten-test
`tests/mhb_planetary_lunar.rs`, a `lib.rs` module export and
documentation/README link. Scientific constants only; no external
SOFA/ERFA/Swiss/NAIF source, binary files, private data, imported
periodic tables, new dependencies, or deployment changes.
The CPAL-1.0 notices, repository `LICENSE` and `NOTICE`,
and required **Powered by VirgoStars**
(https://www.virgostars.it/) attribution remain unchanged.
