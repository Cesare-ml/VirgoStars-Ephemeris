# Lunar-solar fundamental arguments — preparation for IAU 2000A nutation

**2026-11-10 · Experimental original Rust core.** This is the five-argument
Delaunay polynomial stage **only**. It does not implement the IAU 2000A
1365-term periodic nutation series, planetary argument set, the nutation
angles Δψ/Δε, full apparent positions or Swiss Ephemeris compatibility.

## Scientific specification and provenance

The five lunar-solar arguments are `[l, l', F, D, Ω]`:

1. `l`: mean anomaly of the Moon.
2. `l'`: mean anomaly of the Sun.
3. `F`: Moon's mean argument of latitude (mean longitude minus ascending node).
4. `D`: mean elongation of the Moon from the Sun.
5. `Ω`: mean longitude of the Moon's ascending node.

Their fourth-degree time polynomials are published in the
[IERS Conventions (2010), Chapter 5, §5.7.2, Eq. (5.43), p. 23](
https://iers-conventions.obspm.fr/content/chapter5/icc5.pdf#page=23).
The reference time parameter is `T=(JD_TDB-2451545.0)/36525`.
Published coefficients are in arcseconds; the code evaluates each polynomial
and its derivative with simultaneous Horner recurrences, wrapping the angle
only *after* evaluating the polynomial. The derivative is obtained from the
**unwrapped** polynomial so an angle crossing 0/360° does not cause a
spurious velocity discontinuity.

The IERS explicitly notes a subtle source of parity differences in the
paragraph following Eq. (5.43): the IAU 2000A MHB2000 implementation uses
historical rounded constants for `l'` and `D`. Those two constants differ
from Eq. (5.43) by **+2 and -2 microarcseconds** respectively. Therefore the
Rust API requires an explicit `LuniSolarConvention`:

| Convention | `l'` c0, arcsec | `D` c0, arcsec | Intended comparison |
|---|---:|---:|---|
| `Iers2010` | 1287104.793048 | 1072260.703692 | IERS Eq. 5.43 and standalone ERFA `Falp03/Fad03` fixtures |
| `Mhb2000` | 1287104.79305 | 1072260.70369 | Original MHB2000/IAU 2000A historical coefficient convention |

Do **not** silently switch between conventions during later Swiss/SOFA
parity checks. The difference is minute but scientifically real. All five
other polynomial coefficients per argument and all derivatives are identical.
Only these two constant terms differ.

**Numerical stability:** the unwrapped arguments can exceed one billion
arcseconds across the supported interval. Adding a 2-µas correction to that
huge number loses precision in binary64. The Rust implementation therefore
reduces the common polynomial modulo one full turn **first**, then applies
the explicit small MHB2000 constant correction to the reduced angle and
normalizes again. This is mathematically equivalent to adding a constant
before reduction but preserves the meaningful microarcsecond-scale
difference; the regression suite detects this precision loss.

## API and contracts

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::lunisolar_arguments::{
    lunisolar_fundamental_arguments, LuniSolarConvention,
};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let epoch = JulianDate::new(2_451_545, 0.5, TimeScale::Tdb)?;
let args = lunisolar_fundamental_arguments(epoch, LuniSolarConvention::Mhb2000)?;
let (phase_rad, phase_rate) = args.phase([0, 0, 0, 0, 1]);
println!("Ω = {phase_rad} radians, dΩ/dTDB = {phase_rate} rad/day");
# Ok(())
# }
```

Input is a **split Julian date already expressed in TDB**. This strict
choice reflects the original IERS specification; the IERS text says
TT is acceptable *in practice* for nutation arguments because the effect of
TDB–TT is very small, but this API does not silently convert or relabel TT.
For TT callers the conversion or an explicit scientifically documented
approximation must be managed at a higher-level interface.

Output `angles_rad: [f64;5]` is normalized to [0,2π), in Delaunay order;
`rates_rad_per_tdb_day: [f64;5]` is **signed radians/TDB day**. The
`phase([i8;5])` helper combines integer multipliers into one normalized
lunar-solar series phase and its analytic rate, but does **not** evaluate
any nutation amplitude or harmonic term.

The initial engineering guard is `J2000 ±2 TDB Julian centuries`, not an
official IAU validity/error guarantee. Invalid time scale, unrepresentable
split Julian day or epochs outside this bound produce typed errors. This
implementation uses no file, network call, Swiss source, or JPL kernel.

## Verification: external numeric references versus mathematical checks

Eight independent or synthetic regression tests in
`crates/ephemeris-core/tests/lunisolar_arguments.rs`:

- All **five** standalone [ERFA `Fal03`, `Falp03`, `Faf03`, `Fad03`,
  `Faom03` tests](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
  at `T=0.8` centuries. They supply external numeric results; our code
  independently evaluates the IERS polynomial. `Faom03` returns a
  negative signed angle in that particular ERFA fixture; we compare
  equivalent phases modulo 2π. This compares equal *numeric* century
  arguments, not TT→TDB converted epochs, using `2e-12` rad tolerance
  appropriate to the floating-point reduction of large arcsecond angles.
- Exact published J2000 constant terms and first-order TT/TDB-day-scale
  coefficients, evaluated explicitly as **TDB** derivatives in this API.
- Historical MHB2000 ±2 µas offsets are independently checked against
  `Iers2010` and shown not to change instantaneous angular rates.
- Five epochs with independently re-evaluated centered TDB-day angle
  differences, using the shortest signed angular span.
- Fixed order, deterministic integer-combination phase and rate.
- Normalization over a broad ±2-century span, with physical sign checks.
- Strict time-scale and unrepresentable-epoch rejection.
- Explicit boundary checking of the conservative test interval.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test lunisolar_arguments
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The ERFA results validate the **five arguments only**. This does not
validate the full periodic series, IAU 2000A nutation angles, real DE441
planetary vectors, true-of-date frames or Swiss `swe_calc_ut` equality.

## Next planned stages

The IAU 2000A periodic series needs its complete 1365-term coefficient
provenance, the other planetary fundamental arguments, and independently
checked harmonic summation with carefully quantified numeric errors.
Review third-party data rights **before** incorporating any periodic-series
table; published numbers must not be assumed redistributable under CPAL.
After independently checking the computed Δψ/Δε against authoritative
numeric references, connect them to the existing **draft** nutation-angle
matrix PR #51, and the mean-precession PR #48, with explicit TT/TDB
time-derivative reconciliation. No claim of complete implementation until
those stages pass.

## CPAL-1.0 modification record — 2026-11-10

Added original Rust `src/lunisolar_arguments.rs`, its public `lib.rs`
export, standalone `tests/lunisolar_arguments.rs` and this document
plus README link. The only external materials used were public IERS
scientific coefficients and numerical ERFA test results; no third-party
implementation/source code, ephemeris kernel, periodic data table,
new dependency, proprietary asset or production data was introduced.
Preserve the existing CPAL-1.0 source notices and
**Powered by VirgoStars** (https://www.virgostars.it/) attribution.
