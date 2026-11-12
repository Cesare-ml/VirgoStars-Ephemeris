# CIO-based GCRS ↔ CIRS orientation from supplied CIP X, Y, s

**Date:** 2026-11-12. **Status:** original, experimental Rust primitive
in a draft PR to `dev/ephemeris`, not integrated into `main` or the
VirgoStars production backend.

## Why this piece

The planned VirgoStars observer chain is an explicit composition
of independently validated stages, rather than a single hidden
conversion:

```text
WGS84 geodetic → ITRS Earth-fixed XYZ            PR #82
         ↓ W(t) polar motion, including TIO s' PR #84
TIRS terrestrial intermediate XYZ
         ↓ R3(-ERA(UT1)) Earth rotation          PR #83 / #75
CIRS celestial intermediate XYZ
         ↓ Q(t) orientation / CIO X, Y, s       THIS PR
GCRS Earth-centered celestial XYZ
         ↓ reviewed time scales, DE441 target/center choice,
           topocentric subtraction and apparent corrections
         → actual observation
```

**This is not yet an integrated one-call observer-to-DE441 API**:
all listed stages are being developed separately on *draft PRs*;
their time scales, reference origins, rates and accuracy must be
reconciled and validated before composition. In particular neither
a raw WGS84 ECEF vector nor a CIRS vector is automatically in ICRF.

## Input and output contracts

`gcrs_to_cirs_cio_matrix(cip_x, cip_y, cio_s_rad)`
returns a **row-major, proper orthogonal 3×3 matrix** which multiplies
Cartesian **GCRS column vectors** into CIRS Cartesian components.

- `cip_x` and `cip_y` are the *dimensionless Cartesian components*
  of the north Celestial Intermediate Pole unit vector **in GCRS**,
  at an implied but externally specified physical TT epoch. Small
  values are numerically near radians but are *not spherical angles*.
  The API requires finite values with `X²+Y² < 1`, and takes
  `Z = +sqrt(1-X²-Y²)` (the conventional north-pole branch).
- `cio_s_rad` is the supplied **CIO locator s** at the same
  consistent epoch and celestial orientation model, in finite
  **radians [-π,π]**. No implicit wrapping, model calculation or
  time conversion is performed.
- The matrix input does not identify the TT epoch or IAU model, so
  consistency and provenance **must** be established by the caller.
- `gcrs_to_cirs_position` and `cirs_to_gcrs_position` accept
  finite Earth-centered Cartesian vectors in **kilometers**,
  apply the matrix or its transpose, and return kilometers.
  The two operations are **position-only** transformations.

## Independent geometric construction

Define `Z=sqrt(1-X²-Y²)` and `a=1/(1+Z)`. The shortest
right-handed rotation carrying the north CIP `[X,Y,Z]` onto the
intermediate +Z axis has rows

```text
B0 = [1-aX², -aXY, -X]
B1 = [-aXY, 1-aY², -Y]
B2 = [X, Y, Z]
```

The CIO locator orients the intermediate equatorial axes by s:

```text
C0 =  cos(s) B0 - sin(s) B1
C1 =  sin(s) B0 + cos(s) B1
C2 =  B2

p_CIRS = C · p_GCRS
p_GCRS = Cᵀ · p_CIRS
```

This is the **celestial-to-intermediate** CIO-based matrix commonly
denoted `RC2I` (the transpose of `Q(t)` in the complete
ITRS→GCRS chain). A positive s rotates the initial GCRS +X
components toward CIRS +Y when X=Y=0. The third row is the
GCRS unit CIP, independent of s.

The above geometry is original and does not copy or translate
SOFA/ERFA source code. The CIO angle itself must come from
a separately implemented and validated physical series; setting
it to zero without an explicit approximate-mode policy is not
the full IAU 2000A/2006 celestial orientation.

## Independent SOFA/ERFA fixture

The published `t_c2ixys` fixture in ERFA's
`src/t_erfa_c.c` supplies:

```text
X =  0.5791308486706011000e-3
Y =  0.4020579816732961219e-4
s = -0.1220040848472271978e-7 rad

RC2I expected, row-major:
[+0.9999998323037157138, +0.5581984869168499149e-9, -0.5791308491611282180e-3]
[-0.2384261642670440317e-7, +0.9999999991917468964, -0.4020579110169668931e-4]
[+0.5791308486706011000e-3, +0.4020579816732961219e-4, +0.9999998314954627590]
```

Tests compare all **nine published values** with an absolute
`2e-14` tolerance after rounding the literals to representable
`f64` precision. This checks an **independent matrix fixture**,
not the accuracy of the future TT→X/Y/s calculation or
DE441/GCRS consistency.

Additional tests cover CIP-to-intermediate-pole alignment, identity,
CIO sign, transpose roundtrips, orthogonality/determinant,
norm preservation, invalid/nonfinite inputs and overflow.

## Excluded science and integration gates

This PR intentionally provides **no**:
- calculation of CIP `X,Y` or CIO `s` from an epoch (requires
  reviewed IAU 2006/2000A precession–nutation, CIO locator series
  and the correct TT time scale);
- derivative `dC/dTT`, Earth spin rate or observer velocity:
  simply rotating a velocity is **not** a correct moving-frame
  transformation;
- polar-motion `W(t)`, UT1/ERA `R(t)`, terrestrial realization
  or time-scale conversion;
- observer-origin/DE441 geocentric target composition, barycentric
  light-time, stellar aberration, parallax or apparent horizon;
- complete Swiss Ephemeris compatibility or astrological houses.

These are separate acceptance gates and must not be silently
replaced with arbitrary approximations or an unreported fallback.

## Rust call example

```rust
use virgostars_ephemeris_core::cio_orientation::{
    gcrs_to_cirs_cio_matrix, cirs_to_gcrs_position
};

// Illustrative externally supplied X/Y/s, not a TT epoch calculation.
let x = 0.579_130_848_670_601_1e-3;
let y = 0.402_057_981_673_296_1e-4;
let s = -0.122_004_084_847_227_2e-7;
let c2i = gcrs_to_cirs_cio_matrix(x, y, s)?;
let north_in_gcrs = cirs_to_gcrs_position([0.0, 0.0, 1.0], x, y, s)?;
println!("C2I={c2i:?}, CIP={north_in_gcrs:?}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## References, CI and licensing

- [IERS Conventions (2010), Technical Note 36, Chapter 5](https://iers-conventions.obspm.fr/conventions_material.php)
- [ERFA independently published test output, `t_c2ixys`](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
- [ERFA `c2ixys` input/output convention](https://github.com/liberfa/erfa/blob/master/src/c2ixys.c)

Proposed read-only CI verification:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

This original source and its tests carry the CPAL-1.0 Exhibit A
header, with existing VirgoStars attribution unchanged. No
SOFA/ERFA/Swiss implementation, third-party data, JPL kernels,
new dependencies, release or production migration are included.
