# Equinox-based nutation matrix from independently supplied angles

**2026-11-10 — Small independently implemented Rust stage. This is NOT an IAU 2000A or IAU 2000B nutation-series evaluator.**

## Purpose

`crates/ephemeris-core/src/nutation_matrix.rs` converts known nutation
angles into the passive rotation from the **mean equator/equinox of date** to
the **true equator/equinox of date**. In the conventional equinox-based
formulation (IERS Conventions 2010, Chapter 5), the matrix is

```text
N = Rx[-(eps_A + delta_epsilon)] × Rz[-delta_psi] × Rx[eps_A]
```

where all matrices use passive Cartesian rotations, `eps_A` is mean
obliquity, `delta_psi` is nutation in longitude and `delta_epsilon` is
nutation in obliquity, all **radians**.

The full analytic TT-time derivative is calculated by the product rule.
For `N = A×B×C`:

```text
dN/dTT = (dA/dTT)×B×C + A×(dB/dTT)×C + A×B×(dC/dTT)
```

Thus the derivative includes all three independent angular rates, and the
changing mean obliquity contributes even when the nutation angles are not
constant. Matrix and derivative are returned together as `NutationMatrix`.

## Input contract and limitations

`nutation_matrix_from_angles(NutationAngles)` accepts:

| Input field | Definition | Unit |
|---|---|---|
| `mean_obliquity_rad` | Mean ecliptic-to-equator obliquity of date | rad |
| `mean_obliquity_rate_rad_per_tt_day` | Its TT derivative | rad/TT day |
| `delta_psi_rad` | Nutation in longitude, equinox-based | rad |
| `delta_psi_rate_rad_per_tt_day` | Its TT derivative | rad/TT day |
| `delta_epsilon_rad` | Nutation in obliquity | rad |
| `delta_epsilon_rate_rad_per_tt_day` | Its TT derivative | rad/TT day |

All values **must refer to the same TT epoch**. This API does **not**
accept an epoch because it never calculates any time-dependent astronomical
angle internally. There is no TT/UTC/TDB conversion, table of fundamental
arguments, IAU 2000A/2000B periodic series or hidden approximation.

The input mean-obliquity formula itself is being developed in **draft PR
#48**, and is **not** assumed present on the base branch. Future integration
should pass its output explicitly rather than duplicating that calculation.

**Output:** `rotation` is an orthonormal, dimensionless passive 3×3 matrix.
`rate_per_tt_day` is its analytical derivative per **TT day**. A future
date-frame composition with the **PR #48** IAU 2006 precession matrix will
require normal branch integration and consistent GCRS/mean/true conventions.
The **PR #47** moving-frame state transformer expects `dR/d(observation TDB
day)`, **not** the TT-day derivative returned here; a validated TT↔TDB
derivative rule and frame-tagged state types are required before connecting
the routines.

Guarded inputs refuse nonfinite numbers, mean obliquity outside
`0 < eps_A < pi/2` and |nutation angles| ≥0.1 rad. These generous
engineering guards are **not** a physical accuracy certificate or official
IAU validity limits; they protect against unit confusion and absurd data.

## Independent numerical reference

Six offline integration tests in
`crates/ephemeris-core/tests/nutation_matrix.rs` exercise the new primitive.

The **nine matrix entries** are compared to the independent public
[ERFA `t_numat` numerical fixture](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c)
for

```text
eps_A = 0.4090789763356509900 rad
delta_psi = -0.9630909107115582393e-5 rad
delta_epsilon = 0.4063239174001678826e-4 rad
```

with a tolerance of `2e-15` for each dimensionless entry. No ERFA/SOFA
source code is copied, linked, wrapped or installed. Other tests cover
zero nutation even when mean obliquity varies, finite-difference checks of
the full matrix TT-time derivative, proper rotation/orthogonality and tangent
constraint, and invalid numeric/physical input.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test nutation_matrix
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The ERFA fixture validates **the geometry for given angles**, not that
VirgoStars computes those angles correctly for a requested date. No
independent full IAU 2000A coefficients, actual nutation angles, DE441/JPL
positions, or Swiss `swe_calc_ut` parity are provided by this change.

## Required next stages

- Implement the **full licensed/provenance-reviewed IAU 2000A nutation
  angle series** in original Rust, with explicit 2000A versus any truncated
  model; do not silently call a truncated series 2000A.
- Validate at multiple dates against independent ERFA/SOFA numerical
  nutation-angle reference fixtures (including derivatives where available).
- Compose with IAU 2006 precession (draft PR #48), account for TT/TDB
  derivative conventions, and validate a complete apparent-of-date
  model against independently pinned astronomical vectors.
- Do not put this incomplete output into production or label it fully
  apparent/Swiss-equivalent.

## References and license record

- [IERS Conventions (2010), Technical Note 36, Chapter 5](https://iers-conventions.obspm.fr/content/tn36.pdf).
- [ERFA independent numerical `t_numat` regression](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
- [IAU SOFA documentation](https://www.iausofa.org/).

**2026-11-10 CPAL modification record:** added original Rust
`src/nutation_matrix.rs`, exposed module from `src/lib.rs`, added six
synthetic/independent-numeric tests `tests/nutation_matrix.rs`, linked the
documentation from README. No external code (Swiss, SOFA, ERFA, NAIF), no
ephemeris files, private datasets or dependencies incorporated.
All CPAL-1.0 notices and **Powered by VirgoStars**
(https://www.virgostars.it/) attribution preserved.
