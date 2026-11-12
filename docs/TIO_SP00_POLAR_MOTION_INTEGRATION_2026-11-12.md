# Integration of IERS approximate TIO s′(TT) with polar motion W(t)

**Date:** 2026-11-12. **Status:** experimental, draft PR, **position-only**.
No package release, protected-`main` merge, cross-repo change or
VirgoStars production deployment.

## Provenance and feature dependencies

This integration branch is based on the **unmerged** observer integration
branch in PR #89 (`3be777c186c8ebe26f01db1d69a06c99f4d342e0`),
which already contains the original ITRS→TIRS polar-motion
implementation and regression tests from the still-open PR #84.

Three original files are imported unchanged by Git blob SHA from the
separate draft PR #100 (head
`54963d4080d80ae6b89e5e39a04ab79f46b0f7f2`):

- `crates/ephemeris-core/src/tio_locator.rs`: optional bounded
  conventional TT-based SP00 approximation;
- `crates/ephemeris-core/tests/tio_locator.rs`: eight independent
  unit/scale/edge-case regressions;
- `docs/TIO_LOCATOR_SP00_TT_2026-11-12.md`: science and provenance.

These component branches and PRs remain **unchanged and unmerged**.
The integration branch reconciles `lib.rs` exports and adds the
original `tio_polar_motion` adapter, tests and this document.

## New opt-in API

```rust
use virgostars_ephemeris_core::calendar::{JulianDate, TimeScale};
use virgostars_ephemeris_core::tio_polar_motion::
    itrs_to_tirs_polar_motion_with_approximate_sp00;

let epoch_tt = JulianDate::new(2_453_737, 0.0, TimeScale::Tt)?;
let xp_rad = 2.550_602_38e-7;
let yp_rad = 1.860_359_247e-6;
let composed = itrs_to_tirs_polar_motion_with_approximate_sp00(
    epoch_tt,
    xp_rad,
    yp_rad,
)?;
let itrs_to_tirs_w = composed.itrs_to_tirs;
let approximate_sp = composed.locator.sp_rad;
println!("{itrs_to_tirs_w:?}, s′={approximate_sp}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

**Inputs:**
- `epoch_tt`: tagged `TimeScale::Tt` **split Julian date**.
  The module does **not** convert UTC, UT1, TAI or TDB to TT.
- `xp_rad` and `yp_rad`: **independent**, real polar-motion EOP
  parameters in **radians** for the **same physical epoch**. The
  caller must source them and verify that their EOP time stamps
  match the supplied TT epoch. Neither zero values nor an EOP
  series are assumed by this function.

The function first obtains `s′` through
`approximate_tio_locator_sp00(epoch_tt)`, which uses

```text
T = (JD_TT - 2451545.0) / 36525.0

s′_approx = -47 * 10^-6 arcseconds * T
```

with an explicit ±2-Julian-century software guard. It then calls
`itrs_to_tirs_polar_motion_matrix(xp_rad, yp_rad, s′_approx)`
from the original #84 implementation.

**Outputs:**
- `locator.sp_rad`: the **conventional approximate** terrestrial
  intermediate-origin locator (in **radians**);
- `locator.secular_rate_rad_per_tt_day`: **only the mathematical
  derivative** of that linear approximation, not a complete Earth
  orientation rate or inertial observer velocity;
- `itrs_to_tirs`: 3×3 row-major ITRS→TIRS orthogonal matrix acting
  on Cartesian column positions, consistent with the parent polar
  motion API's signs and units.

Typed errors preserve the exact failure of the TT-only SP00 module
or the independently sourced xp/yp polar-motion validation.

## Independent numerical integration fixture

The published SOFA/ERFA test `t_pom00` gives nine matrix elements
of the **TIRS→ITRS** matrix `rpom` for:

```text
xp =  2.55060238e-7 radians
yp =  1.860359247e-6 radians
sp = -0.1367174580728891460e-10 radians
```

The conventional SP00 formula at **TT JD 2453736.5**
(`JulianDate::new(2453737, 0.0, TimeScale::Tt)`) gives **the
same published `sp` value** (s′). The integration test verifies
the match to a 1e-22-radian tolerance. It then verifies all nine
computed elements of the ITRS→TIRS matrix against the **transpose**
of the separately published `t_pom00` values to 1e-15.

Other tests check the independent `t_sp00` numerical oracle at
JD 2452541.5, correct signs when xp/yp are explicitly zero,
compositional identity across epochs and pole inputs, rejection
of UT1/TAI/TDB labels, rejection outside the ±2-century guard,
invalid radians/nonfinite EOP input, orthogonality and position
roundtrip/distance preservation.

These fixtures establish **numeric implementation correctness of
the specified SP00 approximation and IERS polar-motion composition**.
They do **NOT** validate a physically matched TT+EOP observation,
observed xp/yp, a complete CIO/CIP series, reference realization,
barycentric-to-geocentric frame dynamics, or actual DE441/JPL
ephemeris accuracy.

## Why this is intentionally optional

The parent polar-motion module always accepts an **explicit**
s′. It remains the preferred entry point when a validated higher
accuracy TIO locator is available. This integration creates an
**opt-in alternative**, never a silent default or a replacement
for measured/modeled EOP. It does **not** compute the celestial
CIO locator s, precession/nutation, TT↔UT1↔TDB conversions, ERA,
observer velocity, or apparent star/planet positions.

The separate #89 observer API requires independent UT1, xp/yp,
CIP X/Y and CIO s; it can only receive this approximate s′ if
a caller explicitly selects and supplies `composed.locator.sp_rad`.
A complete science-validated observer pipeline still needs
actual same-physical-epoch inputs and independent JPL/Horizons
and IERS/SOFA validation.

## Validation and attribution

The existing repository CI (read-only, authorized by `AGENTS.md`)
should verify:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Original source/test files retain the project **CPAL-1.0 Exhibit A
notices** and required VirgoStars credits. The implementation is
original mathematical code, using **numerical ERFA test values only**
instead of copying external astronomy source code. No binary
kernels, external dataset, new dependency, or GitHub Actions
workflow change is included.

**Sources:** [IERS Conventions 2010, chapter 5](https://iers-conventions.obspm.fr/conventions_material.php),
[ERFA `t_sp00` and `t_pom00` numerical tests](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
