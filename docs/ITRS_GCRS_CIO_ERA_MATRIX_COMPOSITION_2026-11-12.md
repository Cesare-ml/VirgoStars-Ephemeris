# Fixed-epoch ITRS ↔ GCRS CIO/ERA/polar orientation composition

**Date:** 2026-11-12. **Status:** original experimental Rust matrix
composition under review in a draft PR to `dev/ephemeris`.
Not integrated on `main` or in VirgoStars production.

## Motivation and relation to concurrent PRs

Multiple standalone observer/orientation components already exist on
separate, still-unmerged feature branches:

- PR #82 supplies a WGS84 geodetic observer ECEF vector in meters.
- PR #84 supplies the ITRS→TIRS polar-motion matrix W(t), with xp,yp,s′
  **externally supplied** and a published SOFA/ERFA `pom00` fixture.
- PR #83 supplies TIRS→CIRS IAU Earth Rotation Angle and spin-only
  velocity, with ERA and its rate **externally supplied**.
- PR #86 supplies the GCRS→CIRS celestial-intermediate C matrix,
  with CIP X,Y and CIO locator s **externally supplied**.
- PR #75 supplies an IAU 2000 ERA primitive from UT1.

The integration required for a *position-only* physical rotation is:

```text
p_ITRS  --W(t)-->  p_TIRS
              --Rz(+ERA)--> p_CIRS
              --C(t)ᵀ--> p_GCRS
```

For the inverse, celestial→terrestrial rotation:

```text
p_ITRS = W(t)ᵀ · Rz(-ERA) · C(t) · p_GCRS
```

Here `Rz` denotes the ordinary **active** counterclockwise
matrix for column vectors. In IERS/SOFA passive notation
`R3(ERA)` represents the same CIRS→TIRS step as active
`Rz(-ERA)`. `W` in this API is ITRS→TIRS, so it must be
**transposed** to obtain the SOFA/ERFA `RPOM`, which is TIRS→ITRS.
Likewise `C` is GCRS→CIRS (SOFA `RC2I`).

This PR is intentionally independent of all five unmerged feature
branches: its parameters are already-computed 3×3 matrices and
a normalized ERA. It **does not recalculate** polar motion, Earth
rotation, precession, nutation or CIO physics.

## Exact Rust API

`compose_gcrs_to_itrs(itrs_to_tirs_w, era_rad, gcrs_to_cirs_c)`
returns a `ComposedOrientation` holding the row-major
`gcrs_to_itrs` 3×3 matrix. Every matrix multiplies a **Cartesian
column vector**.

- `itrs_to_tirs_w`: 3×3 finite proper orthogonal ITRS→TIRS matrix
  for the intended Earth orientation and epoch; not RPOM directly.
- `era_rad`: finite IAU 2000 Earth Rotation Angle [0, 2π) radians
  at **UT1**. Supplied by the caller. No UT1 conversion occurs here.
- `gcrs_to_cirs_c`: 3×3 finite proper orthogonal GCRS→CIRS matrix
  derived from an independently validated CIP/CIO model at **TT**.
- Each input matrix must be right-handed with row-orthogonality,
  unit row norms and determinant +1 to within an explicit numerical
  tolerance of `1e-10`. This check does **not** prove that the
  matrix corresponds to the specified epoch or physical model.
- The returned matrix and its transpose can be applied through
  `gcrs_to_itrs_position([km;3])` and
  `itrs_to_gcrs_position([km;3])`. Position inputs/outputs are
  finite **kilometers**, Earth-centered in the named frame, with
  typed errors on invalid inputs and non-finite results.

The individual upstream transformations remain separately testable.
No default epoch, zero polar motion, zero CIO locator, sidereal-angle
approximation, UTC=UT1, missing EOP substitution or implicit
network lookup is performed.

## Independent numerical fixture

The independently published `t_c2tcio` test in
[ERFA `src/t_erfa_c.c`](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
provides all nine `RC2T` values from a known input
`RC2I` (GCRS→CIRS), `RPOM` (TIRS→ITRS) and
`ERA=1.75283325530307 radians`.

Its expected row-major GCRS→ITRS matrix, after rounding constants
to representable `f64`, is:

```text
[-0.18103321283071105,   0.983476980693847,    0.00006555535638685467]
[-0.9834768134135997,   -0.18103322036494485,  0.0005749801116141107]
[ 0.0005773474014081407, 0.00003961832391772659, 0.9999998325501692]
```

The Rust regression reproduces the **nine independently published
elements**, with absolute tolerance `1e-12`; no SOFA/ERFA
source implementation is copied. The test also intentionally
transposes the published RPOM before passing it as W, which catches
a common IERS sign/order error.

Seven other deterministic test groups cover identity, ERA-only
direction/sign, W versus RPOM direction, inverse roundtrips,
distance preservation, rejection of improper/singular/nonfinite
matrices and invalid angle or vector inputs/overflow.

These are independent **rotation** validation results; no full
physical Earth-orientation from EOP/UT1/TT or DE441/JPL/Horizons
position parity has been established.

## Intentional exclusions and next science gates

The composition **cannot compute** its inputs. Before use in
VirgoStars, a separate integration must supply W, ERA and C for
the **same physical epoch** and consistent ITRS terrestrial
realization; verify UTC→TAI→TT and UTC→UT1, CIP/CIO series,
polar-motion EOP including s′ and any required station motion.

The output is **position-only**. For observer velocities,
the derivative of W(t), ERA(t), C(t) and observer station motion
must be accounted for using a single declared time scale.
Simply multiplying an ITRS velocity by this matrix is **not**
an inertial observer velocity. DE441’s barycentric vs geocentric
origins, target-center semantics, light-time and other physical
corrections must be handled separately.

Consequently this PR does **not** implement:
- a one-call, validated WGS84→GCRS/DE441 observer solution;
- IERS EOP acquisition/interpolation or UTC/UT1/TT/TDB conversion;
- apparent topocentric parallax, solar/lunar rise/set or houses;
- an independently validated Swiss replacement.

## Example usage

```rust
use virgostars_ephemeris_core::orientation_chain::compose_gcrs_to_itrs;

// Illustration only. Supply physically consistent W, ERA and C for
// an actual observation; identity matrices here are not valid EOP.
let identity = [
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
];
let orientation = compose_gcrs_to_itrs(identity, 1.0, identity)?;
let gcrs_position_km = orientation.itrs_to_gcrs_position([6378.137, 0.0, 0.0])?;
println!("{gcrs_position_km:?}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification / legal notices

Read-only CI commands (when permitted by existing repository settings):

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

This independently authored implementation retains the CPAL-1.0
source Exhibit A notices and VirgoStars required attribution.
No SOFA, ERFA, Swiss, NAIF code is included, no JPL binary
or external EOP data is redistributed, and no package release,
auto-deployment or production migration is requested.

**Standards:** [IERS Conventions (2010), chapter 5](https://iers-conventions.obspm.fr/conventions_material.php);
[ERFA, independently published `t_c2tcio` test values](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
