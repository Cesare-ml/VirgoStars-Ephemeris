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

## Follow-up: consistent true-obliquity data for moving ecliptic frames — 2026-11-12

The original `nutation_with_true_obliquity(NutationAngles)` function
returns `NutationWithTrueObliquity`, bundling the existing exact
**mean-equatorial → true-equatorial** nutation matrix **N** with its
true obliquity `ε_true` and **TT-day rate**.

```text
ε_true                  = ε_mean + Δε
dε_true / dTT           = dε_mean / dTT + d(Δε) / dTT
N                       = Rx(-ε_true) Rz(-Δψ) Rx(ε_mean)
R_true_equatorial       = N P            (P independently supplied)
R_true_ecliptic         = Rx(+ε_true) N P
dR_true_ecliptic / dTDB = (dR/dTT) × dTT/dTDB
```

**Why provide a single tuple?** The moving ecliptic-of-date frame
builder on **independent, unmerged PR #47** accepts the nutation
matrix and the true ecliptic tilt with its rate as separate inputs.
Passing these quantities from unrelated epochs or formula variants
would silently produce incorrect latitudes, longitudes and
especially **moving-frame angular velocities**. The new tuple
guarantees they come from *the same, already validated caller
angle set*, although it cannot guarantee that the externally
supplied angles are physically right at any date.

No precession, complete IAU 2000A series, astronomical time-scale
model, JPL file, true-frame `Rdot` or Swiss replacement has been
added to this PR. A downstream caller must still obtain genuine
precession P from draft #48, independently derive the physical
nutation angles with cleared coefficient provenance, and supply
the correct **TT epoch and dTT/dTDB** conversion when using #47.

The tuple is usable as:

```rust
use virgostars_ephemeris_core::nutation_matrix::{
    nutation_with_true_obliquity, NutationAngles,
};

# fn example(angles: NutationAngles) -> Result<(), Box<dyn std::error::Error>> {
let state = nutation_with_true_obliquity(angles)?;
let n_tt = state.nutation.rotation;
let n_dot_tt = state.nutation.rate_per_tt_day;
let true_tilt = state.true_obliquity_rad;
let true_tilt_rate_tt = state.true_obliquity_rate_rad_per_tt_day;
# let _ = (n_tt, n_dot_tt, true_tilt, true_tilt_rate_tt);
# Ok(())
# }
```

The original `nutation_matrix_from_angles` public API is preserved.
The new adapter calls it directly, with no duplicated nutation
algebra, and validates the **derived** obliquity lies strictly in
`(0, π/2)`. It uses a new error `InvalidTrueObliquity` if
`ε_mean + Δε` lies outside this interval, despite separately
guarded input components. The original matrix evaluator is
also hardened: even **finite** inputs must not produce
nonfinite output matrix entries or TT derivatives. Such cases
return a separate `ArithmeticOverflow` error, never a plausible
looking matrix containing infinities or NaNs.

### Exact cancellation invariant

The three-factor equinox-based formulation has a useful, independently
checkable identity:

```text
E N = Rx(+ε_true) Rx(-ε_true) Rz(-Δψ) Rx(ε_mean)
    = Rz(-Δψ) Rx(ε_mean)
```

This means **Δε cancels** from the combined true-equatorial
to conventional equinox-based ecliptic-plane orientation, and
its TT derivative cancels too:

```text
d(E N)/dTT = Edot N + E Ndot
```

This is an exact result **of this equinox-based rotation convention**,
not a claim that astronomical true ecliptic geometry is independent
of every definition of ecliptic pole, nor an independent physical
validation of the IAU precession/nutation models. The cancellation
is particularly sensitive to sign conventions and the consistent
time derivative of the true-obliquity tilt.

The **nine** new offline Rust tests in
`tests/nutation_true_obliquity.rs` verify the original matrix is
unchanged; exact sum and TT-day scalar rate; independent
time differences of ε; E×N cancellation of Δε; changes in Δε
and its TT rate do not corrupt the final E×N matrix or its
derivative; **analytical Edot×N + E×Ndot** matches numerical
TT differences; zero-nutation handling; new physical
ε_true bounds; invalid angle propagation; and finite-input
arithmetic overflow rejection.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test nutation_true_obliquity
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

All fixture angles are synthetic except the pre-existing
independent ERFA matrix numbers in the older test; there are no
physically verified real-date nutation angles or planetary positions
in this change. **Software CI passing will not mean Swiss parity**.

**2026-11-12 CPAL-1.0 modification record:** extended
`crates/ephemeris-core/src/nutation_matrix.rs` with a
paired nutation/true-obliquity output and strict nonfinite/physical
output checks; added original
`crates/ephemeris-core/tests/nutation_true_obliquity.rs`
with 9 regressions; extended this documentation and README.
Existing CPAL Exhibit A source headers and **Powered by VirgoStars**
<https://www.virgostars.it/> attribution remain. No external
program source, IAU coefficient table, ephemeris binary, new
dependency, package release, merge or deployment.
