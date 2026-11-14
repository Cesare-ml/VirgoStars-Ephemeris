# IERS polar motion: ITRS → TIRS — 2026-11-12

**Status:** original, experimental Rust primitive in a draft PR. Not an
end-to-end topocentric service, not integrated in VirgoStars production.

## Architecture and physical meaning

IERS Conventions (2010), Technical Note 36, chapter 5 §5.4.1,
equations 5.1 and 5.3, separate the transformations:

```text
WGS84 geodetic observer → Earth-fixed XYZ (PR #82, meters → km)
       ↓ Adopt an ITRS terrestrial realization / epoch
ITRS ── W(t) polar motion + TIO locator ──→ TIRS   ← THIS PR
TIRS ── ERA(UT1), R(t) ──→ CIRS                     ← PR #83
CIRS ── Q(t), CIP/CIO orientation ──→ GCRS          ← FUTURE
GCRS / ICRF-compatible observer state + DE441 target state
       ↓ Topocentric parallax, apparent/horizon corrections ← FUTURE
```

A WGS84 ECEF vector is an Earth-fixed Cartesian position; real precision
depends on its ITRS realization, measurement epoch, geoid/ellipsoid height
and possible station drift. This PR **only** handles the polar-motion
rotation, and **never** labels TIRS as GCRS/ICRF. The outputs must not be
subtracted directly from the DE441 target vectors.

## Equations and rotation convention

IERS TN36 Eq. 5.3, for a column position vector:

```text
position_TIRS = W(t) position_ITRS
W(t) = R3(−s′) R2(xp) R1(yp)  [IERS passive-angle convention]
```

xp and yp are coordinates of the Celestial Intermediate Pole (CIP)
in the ITRS, supplied as **radians**. s′ is the
Terrestrial Intermediate Origin (TIO) locator in **radians**.
The use of R1, R2, R3 must follow the IERS/SOFA passive convention;
the sign of each angle matters.

The inverse, for position **only**, is the transpose Wᵀ. SOFA/ERFA
`eraPom00` computes the inverse polar-motion matrix (TIRS→ITRS)
used by the celestial-to-terrestrial coordinate transform; its
published numeric regression must be **transposed** when compared
with this module's ITRS→TIRS `W`.

The module provides:

- `itrs_to_tirs_polar_motion_matrix(xp_rad,yp_rad,sp_rad)`:
  orthogonal matrix W(t).
- `itrs_to_tirs_position(position_itrs_km,xp_rad,yp_rad,sp_rad)`:
  position-only ITRS→TIRS in **kilometers**.
- `tirs_to_itrs_position(position_tirs_km,xp_rad,yp_rad,sp_rad)`:
  inverse position-only transform.

Both frames share the terrestrial/geocentric origin; the physical
length is preserved by this rotation. The angles are not silently
converted from arcseconds/mas, degrees, UTC or TT. Callers must
obtain EOP xp, yp and an epoch-consistent s′ from validated sources
and convert their units **explicitly**. No interpolation, tidal model,
libration, geodetic correction, EOP sampling or s′(TT)
approximation is performed.

The deliberately conservative API guard requires each angle finite
and `abs(angle) <= 0.01 rad` (about 0.57°). This is **not an IERS
physical bound**, merely fail-closed protection against wrong units.
Nonfinite position inputs and outputs return explicit errors.

**No velocity** is returned. Position-only W(t) does not supply a
site's derivative; evaluating an ITRS→TIRS velocity requires
time derivatives of xp, yp, s′ and any site motion. The downstream
ERA-only velocity of PR #83 similarly excludes these effects;
neither module provides a full GCRS velocity.

## Independent reference fixture

ERFA `t_pom00` (published upstream test of `eraPom00`):

```text
xp =  2.55060238e−7 rad
yp =  1.860359247e−6 rad
s′ = −0.1367174580728891460e−10 rad
```

It publishes `RPOM` from TIRS→ITRS:

```text
[ 0.9999999999999674721,   -1.367174580728846989e−11,  2.550602379999972345e−7 ]
[ 1.414624947957029801e−11, 0.9999999999982695317, -1.860359246998866389e−6 ]
[-2.550602379741215021e−7,  1.860359247002414021e−6, 0.9999999999982370039]
```

The Rust integration test compares W(t) against **transpose(RPOM)**
at an absolute tolerance of `1e-15` per matrix entry, retaining
normal f64 rounding. These are numerical reference outputs only;
no SOFA/ERFA source code or runtime is copied or linked.

Further tests cover exact identity for zero EOP, CIP offset signs,
nonzero s′ direction, inverse round-trips over terrestrial positions
and exaggerated-but-bounded EOP, W orthogonality/determinant,
invalid/nonfinite angles/positions and overflow handling. This is
**isolated matrix validation**, not a numerical proof of the
complete ITRS→GCRS/Earth-orientation or JPL/Horizons pipeline.

References:

- [IERS Conventions 2010, chapter 5, Eq. 5.3](https://iers-conventions.obspm.fr/conventions_material.php)
- [ERFA independent t_pom00 regression](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
- [IERS Conventions overview](https://www.iers.org/iers/en/dataproducts/conventions/conventions)

## Example

```rust
use virgostars_ephemeris_core::polar_motion::{
    itrs_to_tirs_position, tirs_to_itrs_position
};

let site_itrs_km = [6378.137, 0.0, 0.0];
let xp_rad = 2.55060238e-7;
let yp_rad = 1.860359247e-6;
let sp_rad = -1.3671745807288915e-11;
let site_tirs_km =
    itrs_to_tirs_position(site_itrs_km, xp_rad, yp_rad, sp_rad)?;
let recovered =
    tirs_to_itrs_position(site_tirs_km, xp_rad, yp_rad, sp_rad)?;
println!("TIRS: {site_tirs_km:?}; ITRS roundtrip: {recovered:?}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Validation

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

Original Rust source and tests use the existing CPAL-1.0 header
and preserve the VirgoStars attribution. No new dependencies, EOP
files, JPL kernels, external algorithm copies, package releases,
deployments or production modifications are introduced.
