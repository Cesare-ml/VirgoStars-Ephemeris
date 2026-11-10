# IAU 2006 GCRS → mean-equator/equinox-of-date bias-precession

**Increment: 2026-11-10. Status: experimentally implemented, cross-checked against the independent ERFA `Pmat06` matrix fixture. Not true-of-date, apparent-of-date, or Swiss Ephemeris parity.**

## Reference and scientific definition

The four Fukushima–Williams angles are the **IAU 2006** precession
parameters from [IERS Conventions (2010), Technical Note 36, Eq. (5.40)](
https://iers-conventions.obspm.fr/content/tn36.pdf).
Their original published arcsecond polynomials, with `T` TT Julian
centuries after J2000 noon (`JD_TT=2451545.0`), are:

```text
gamma_bar = -0.052928 + 10.556378 T + 0.4932044 T^2
           - 0.00031238 T^3 - 0.000002788 T^4 + 0.0000000260 T^5
phi_bar   = 84381.412819 - 46.811016 T + 0.0511268 T^2
           + 0.00053289 T^3 - 0.000000440 T^4 - 0.0000000176 T^5
psi_bar   = -0.041775 + 5038.481484 T + 1.5584175 T^2
           - 0.00018522 T^3 - 0.000026452 T^4 - 0.0000000148 T^5
eps_a     = 84381.406 - 46.836769 T - 0.0001831 T^2
           + 0.00200340 T^3 - 0.000000576 T^4 - 0.0000000434 T^5
```

The last polynomial is **reused from the existing**
`mean_obliquity_iau2006` implementation rather than being duplicated.

We independently form the **passive** orientation, in the sense
`vector(mean-equator/equinox-of-date) = R * vector(GCRS)`:

```text
R(TT) = Rx(-eps_a) · Rz(-psi_bar) · Rx(phi_bar) · Rz(gamma_bar)
```

Here `Rx` and `Rz` are proper, passive right-handed component rotation
matrices; for example `Rz(theta) = [[cosθ, sinθ,0],[-sinθ, cosθ,0],[0,0,1]]`.
The small **frame bias** at J2000 is part of the resulting matrix.
This agrees with the publicly documented [SOFA Fukushima–Williams
definition](https://juliaastro.org/SOFA/stable/api/precession/) without
copying SOFA or ERFA software.

## Exact moving-frame derivative with a strict time contract

Each scientific polynomial is evaluated with simultaneous Horner recurrences
for both its value and derivative. Each elemental rotation also yields an
analytic derivative, with the full product rule

```text
(A·B)' = A'·B + A·B'
```

applied across the four ordered factors. The returned matrices are:

- `rotation`: GCRS to **mean-of-date** Cartesian orientation, unitless.
- `rate_per_tt_day`: derivative of that same matrix with respect to **TT
  days**; each element has units of inverse TT days.

The API is `precession_iau2006::precession_bias_iau2006(JulianDate)`. Inputs
must already be labeled `TimeScale::Tt`. The existing split-JD precision and
same conservative J2000 ±2 TT-Julian-century engineering guard apply.
No TT↔TDB conversion is performed or implied.

**Important:** the separate work in draft PR #47
(`rotating_frame::RotatingFrame`) expects `dR/d(observation TDB day)`,
not `dR/dTT day`; it is **not safe** to pass this matrix derivative
directly to that API. The actual time-argument relationship must be
established and validated first, and the date-frame input/observer convention
must be checked. Similarly, the classic-DE441 ICRF axes must not silently
be treated as fully specified GCRS without verifying frame conventions.

## Independent numerical verification

The integration tests
`crates/ephemeris-core/tests/precession_iau2006.rs` include:

1. **All nine** IAU 2006 bias-precession matrix entries at TT
   `(2400000.5, 50123.9999)`, independently published by the
   [ERFA `eraPmat06` test fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c),
   checked to an absolute `1e-13` (dimensionless).
2. Analytic `dR/dTT` against independently re-evaluated centered finite
   differences across four representative dates, not another derivation
   from the same code.
3. Right-handed orthonormality (`det R=+1`) and the tangent identity
   `Rdot Rᵀ + R Rdotᵀ=0`.
4. Nonzero bias at J2000, rejecting accidental identity at epoch.
5. Explicit non-TT, unrepresentable and out-of-test-range date errors.

These fixtures validate this **specific orientation**, not a finished
planetary ephemeris or its absolute astrometric accuracy. No full JPL
kernel comparison was performed, and no proprietary dataset was incorporated.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test precession_iau2006
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

All checks are also intended for existing read-only GitHub Actions on
Linux, Windows, and macOS, without network ephemeris downloads.

## Still missing before Swiss-style apparent ecliptic positions

This matrix is **mean-of-date, not true-of-date**. IAU 2000A nutation,
gravitational deflection, light-time/aberration pipeline integration,
observer options, frame bias handling in DE441 ICRF/GCRS, TT↔TDB time-rate
conversion, and explicit output frame-tagged types remain separate stages.
No astrological zodiac convention is implemented here.

## CPAL-1.0 modification record — 2026-11-10

Added original Rust `src/precession_iau2006.rs` and integration tests
`tests/precession_iau2006.rs`; refactored the common split-TT epoch
validation in `src/mean_obliquity.rs`, exported the new module via
`src/lib.rs`, and documented the new capabilities. Only public IERS
scientific coefficients and independent ERFA **numeric test values** were
used. No source-code translation, imported software, dependencies, datasets
or binaries from SOFA/ERFA/Swiss/NAIF were added. Existing CPAL-1.0
notices and required **Powered by VirgoStars**
(https://www.virgostars.it/) credits are preserved.
