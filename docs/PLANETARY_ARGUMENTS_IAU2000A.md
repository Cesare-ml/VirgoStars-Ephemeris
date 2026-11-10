# Planetary fundamental arguments F6–F14 for future IAU 2000A nutation

**2026-11-10 · Experimental, original Rust implementation of published scientific polynomials.** This module does **not** evaluate the IAU 2000A lunisolar/planetary harmonic coefficient tables, nutation angles Δψ or Δε, true-of-date planetary positions, or Swiss Ephemeris outputs.

## Scientific source

The canonical specification is [IERS Conventions (2010), Technical Note 36,
§5.7.3, Eq. (5.44)](https://iers-conventions.obspm.fr/content/tn36.pdf);
its published planetary mean longitudes and the general precession in
longitude use **radians** and TDB Julian centuries after J2000 noon:

```text
T = (JD_TDB - 2451545.0) / 36525

F6  Mercury  = 4.402608842 + 2608.7903141574 × T
F7  Venus    = 3.176146697 + 1021.3285546211 × T
F8  Earth    = 1.753470314 +  628.3075849991 × T
F9  Mars     = 6.203480913 +  334.0612426700 × T
F10 Jupiter  = 0.599546497 +   52.9690962641 × T
F11 Saturn   = 0.874016757 +   21.3299104960 × T
F12 Uranus   = 5.481293872 +    7.4781598567 × T
F13 Neptune  = 5.311886287 +    3.8133035638 × T
F14 p_A      = 0.024381750 × T + 0.00000538691 × T²
```

**These are fundamental arguments for a nutation series, not DE441
heliocentric planetary longitudes or geocentric observed positions.**
The derivative of each linear expression is the published slope divided
by 36,525 TDB days; for `p_A`, it is the derivative of the quadratic.
Angles are returned reduced to **[0,2π)**, while rates retain the signed
derivative of the unwrapped polynomial in **rad/TDB day**.

### Explicit historical Neptune distinction

The paragraph following IERS Eq. (5.44) states that the original MHB2000
IAU 2000A planetary-series implementation instead used:

```text
F13 Neptune (MHB2000) = 5.321159000 + 3.812777400 × T
```

Consequently the Rust API requires one of the explicit policies:

| Policy | Neptune expression | Appropriate reference |
|---|---|---|
| `PlanetaryArgumentConvention::Iers2010` | IERS Eq. 5.44 | Standalone ERFA `FaNe03` numerical fixture |
| `PlanetaryArgumentConvention::Mhb2000` | Historical MHB2000 constants | Future exact original IAU 2000A planetary-series convention |

There is **no implicit default or automatic fallback**. The Neptune
difference is much larger than the two ±2-µas lunar-solar constant-rounding
differences addressed in draft [PR #52](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/52).
In addition, IERS identifies **simplified Delaunay variables F1–F5**
for the *planetary* part of the original MHB2000 nutation series; this
module does not implement those simplified arguments. It would therefore
be incorrect to claim exact MHB2000 nutation-series parity even if these
nine polynomials individually match.

## API and time/units contract

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::planetary_arguments::{
    planetary_fundamental_arguments, PlanetaryArgumentConvention,
    NEPTUNE_MEAN_LONGITUDE,
};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let date = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;
let args = planetary_fundamental_arguments(
    date,
    PlanetaryArgumentConvention::Iers2010,
)?;
let neptune_phase = args.angles_rad[NEPTUNE_MEAN_LONGITUDE];
let neptune_rate = args.rates_rad_per_tdb_day[NEPTUNE_MEAN_LONGITUDE];
let (term_phase, term_rate) = args.phase([0, 0, 0, 0, 0, 0, 0, 1, -1]);
println!("{neptune_phase} {neptune_rate} {term_phase} {term_rate}");
# Ok(())
# }
```

The input is a **split JulianDate labeled TDB**; TT, TAI, and UT1 are
rejected with typed errors. No TT↔TDB, UTC or calendar conversion is
performed. The published IERS arguments use TDB centuries; the IERS
notes that TT can be an adequate practical substitute for some applications,
but that substitution must be a higher-level, explicitly documented choice.

Output arrays are ordered `[Mercury, Venus, Earth, Mars, Jupiter, Saturn,
Uranus, Neptune, general precession in longitude]`, corresponding exactly
to F6–F14. The `phase([i8;9])` helper combines nine integer multipliers
to yield one normalized angle and one signed derivative. A **full**
planetary harmonic phase also includes F1–F5 lunar arguments and requires
subsequent integration with the independent [PR #52](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/52).

The initial J2000 ±2 **TDB** Julian centuries guard is a project
testing/precision policy, **not** an official IAU validity guarantee or
measurement of planetary position accuracy.

## Independent numerical fixtures and regression coverage

Eight offline Rust integration tests in
`crates/ephemeris-core/tests/planetary_arguments.rs` include:

1. **Nine independent published ERFA numeric fixtures** for
   `eraFame03`, `eraFave03`, `eraFae03`, `eraFama03`, `eraFaju03`,
   `eraFasa03`, `eraFaur03`, `eraFane03`, and `eraFapa03`, all at the
   same numeric `T=0.8`. Compare phases modulo 2π, absolute tolerance
   1e-12 rad. Source:
   [ERFA `t_erfa_c.c` independent numerical tests](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
   **No ERFA program code is copied or linked.**
2. All eight IERS J2000 base angles/slopes and the quadratic `p_A`
   derivative, independently obtained from the published Eq. (5.44).
3. The historical MHB2000 Neptune alternative changes only that argument's
   angle/rate, without silently altering the other eight.
4. Five different epoch finite-difference checks of all nine analytical
   TDB-day rates; shortest signed angular differences avoid wrap artifacts.
5. Independent nine-multiplier term phase/rate expression.
6. Stable F6–F14 index order, normalized phases and signed rates.
7. Subsecond split-JD input sensitivity.
8. Rejection of invalid time scales, unsafe split epochs and dates beyond
   the conservative tested interval.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test planetary_arguments
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These results prove only this **specific set of scientific argument
polynomials**, not the accuracy of the real planetary ephemeris, the
complete 1,365-term nutation series, or compatibility of six Swiss
`swe_calc_ut` output fields.

## Next scientific work

- Integrate F1–F5 with F6–F14 only after draft PR #52 receives review
  and is merged consistently. Preserve the planetary MHB2000 simplified
  lunar-argument convention.
- Audit provenance and licensing of all 678 lunisolar and 687 planetary
  IAU 2000A periodic terms before distributing any coefficient tables.
- Evaluate Δψ, Δε and their time derivatives in Rust; compare multiple
  historical/contemporary dates against independently pinned ERFA/SOFA
  numerical results, keeping the 2000A and 2006/2000A adjustments
  distinctly labeled.
- Then integrate with precession (draft PR #48), angular nutation matrix
  (draft PR #51), observer and correct TT/TDB derivative conventions,
  followed by real NASA/JPL/Swiss planetary vector comparisons.

## CPAL-1.0 modification record

2026-11-10 — Added `crates/ephemeris-core/src/planetary_arguments.rs`,
`tests/planetary_arguments.rs`, the `src/lib.rs` export, this document,
and README cross-reference. All source implementation is original Rust
using **scientific numbers** from IERS and **independent numerical
test values** from ERFA only; no ERFA/SOFA/Swiss/NAIF implementation code,
external ephemeris kernels, periodic-series coefficient tables, proprietary
dataset or new dependency was included. CPAL-1.0 source notices and
the required **Powered by VirgoStars** credit
(https://www.virgostars.it/) are preserved.
