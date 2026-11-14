# Harmonic summation kernel for future IAU 2000A nutation

**2026-11-10 — experimental original Rust arithmetic.** This change implements
a reusable, numerically compensated periodic-series **calculator given a
caller-supplied coefficient table**. It does **not** distribute, infer, fetch,
or validate any of the official IAU 2000A coefficient tables, and it cannot
yet output independently validated IAU 2000A nutation angles.

## Scientific mathematical basis

[U.S. Naval Observatory Circular 179, §5.4.2 and Appendix E](
https://aa.usno.navy.mil/downloads/Circular_179.pdf)
describes the 1,365-term IAU 2000A equinox-based nutation series:
678 lunar-solar terms and 687 planetary terms. Their trigonometric form
may be expressed, using a single term phase φ and Julian centuries T, as

```text
Δψ_i = (S_i + T*Sdot_i) sin φ_i + C_i cos φ_i
Δε_i = (E_i + T*Edot_i) cos φ_i + F_i sin φ_i

φ_i = Σ_j (m_ij * a_j),  φdot_i = Σ_j (m_ij * adot_j)
```

The coefficients are **angular amplitudes**. To avoid the common error
of interpreting raw table units as radians, this API **requires the caller
to supply amplitudes already converted to radians**, and slopes to
radians per Julian century. **No implicit table-unit scale** exists.
The derivatives include **both** the phase movement and secular slope:

```text
d(Δψ_i)/dt = (Sdot_i/36525)*sin φ_i
             + [(S_i+T*Sdot_i)*cos φ_i-C_i*sin φ_i]*φdot_i

d(Δε_i)/dt = (Edot_i/36525)*cos φ_i
             + [F_i*cos φ_i-(E_i+T*Edot_i)*sin φ_i]*φdot_i
```

Here *t* is a **TDB Julian day** and therefore `φdot_i` is in
rad/**TDB day**; this isolated API also uses a strict split Julian date
with `TimeScale::Tdb`. This aligns the mathematical interface with the
TDB-parameterized argument modules under development in draft PRs #52 and
#53. **It does not claim that TDB-day derivatives can directly replace
TT-day derivatives in an authoritative IAU 2006/2000A frame model.**
The [IERS explanation of the time argument](https://www.iers.org/iers/en/service/faqs/thenewiauresolutions/timeargumentforusingiersproducts-104-157)
identifies TT for the full IAU 2000A precession-nutation model. A future
integrator must reconcile the epoch/time derivative conventions, and
validate the real time transformations.

## API and phase ordering

`nutation_harmonics::evaluate_nutation_harmonics<const N:usize>(tdb_epoch,
angles_rad, rates_rad_per_tdb_day, terms)` accepts exactly two model
families:

| Array size | Family | Maximum supplied terms | Required argument order |
|---|---|---:|---|
| `N=5` | Lunar-solar | 678 | `l,l',F,D,Ω` |
| `N=14` | Planetary | 687 | `l,l',F,D,Ω,Mercury,Venus,Earth,Mars,Jupiter,Saturn,Uranus,Neptune,p_A` |

These are the **F1–F14** arguments in IERS order. In USNO Circular 179,
Appendix E, the displayed columns order the planetary and lunar
multipliers differently; a future importer **must explicitly map**
source table columns to these API indices. Never ingest a table's raw
column order without an audited mapping.

- The supplied fundamental angles must already be **finite and
  normalized to [0,2π)**, with finite rates representing derivatives of
  the **unwrapped** angles at the same epoch.
- The inputs `NutationHarmonicTerm<N>` have exactly six trigonometric
  coefficients; fields explicitly name their angle and slope units.
  `N=14` planetary terms should normally have both secular slopes
  zero, but their values are *not* silently forced to zero.
- Source table licensing, term order and correctness are **caller
  responsibilities**. There are currently **no** official term data or
  third-party implementation in this repository.
- A returned `NutationHarmonicSum` has `longitude_rad` and
  `obliquity_rad`, plus their TDB-day rates. It includes
  `terms_evaluated` to make partial tables visible. A partial sum
  must **never** be presented as the full IAU 2000A nutation model.
- Empty tables, more-than-family-sized tables, N other than 5 or 14,
  wrong time scales, nonfinite values, invalid phases, unrepresentable
  dates and arithmetic overflows return explicit typed errors.
- Epochs are initially guarded to J2000 ±2 TDB Julian centuries; this
  is a test/engineering policy, **not** an official IAU accuracy limit.

The future full model must combine independently reviewed sums from
**both** families only after validating that both outputs refer to the
same physical time argument, angular reference system and coefficient
conventions.

### No hidden precision loss

The terms can alternate in sign and differ greatly in magnitude.
Ordinary left-to-right floating-point summation may lose small terms.
The Rust implementation uses **Neumaier compensated summation**
independently for Δψ, Δε, dΔψ/dTDB and dΔε/dTDB; it rejects nonfinite
intermediate/final results rather than emitting NaN or a misleading zero.
Compensation is an arithmetic safeguard, **not** an astronomical
error-bound certificate.

## Deterministic tests

`crates/ephemeris-core/tests/nutation_harmonics.rs` contains **11**
synthetic mathematical tests:

1. A fully hand-calculable single lunisolar harmonic at J2000 with
   explicitly known longitude, obliquity and time derivatives.
2. Centered differences of both derivatives for two terms while
   fundamental angles and amplitudes evolve independently.
3. A fourteen-argument planetary harmonic that depends only on
   `p_A` and checks its longitude/obliquity values and rates.
4. Zero multipliers, distinguishing secular amplitude drift from phase
   motion.
5. Deliberate `1e16+1-1e16` cancellation verifying that compensation
   preserves a small contribution.
6. Angle wrapping across 0/360° without spurious rate changes.
7. Synthetic full-length 678+687 entries, confirming family limits
   **without** claiming real 1,365-term nutation correctness.
8. Missing, oversize and unsupported-width term families.
9. Wrong time scales, oversized Julian dates and test-window boundaries.
10. Nonfinite and unnormalized phases, nonfinite rates/amplitudes.
11. Arithmetic overflow rejected rather than silently returned.

```bash
cargo fmt --all -- --check
cargo test --workspace --locked --test nutation_harmonics
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

**There are no independent physical Δψ/Δε ERFA/SOFA comparison vectors
in this change.** The tests demonstrate the mathematics of supplied
harmonic sums, not numerical parity of any official IAU 2000A model,
astronomical positions, JPL Horizons, or Swiss Ephemeris.

## Coefficient provenance / integration blockers

- [USNO Circular 179 Appendix E](
https://aa.usno.navy.mil/downloads/Circular_179.pdf) describes the
official **678 + 687** term families and their form. Published text
availability does **not** automatically authorize taking entire
tables or software source and redistributing them under CPAL.
- [IERS Conventions (2010), Chapter 5](
https://iers-conventions.obspm.fr/content/tn36.pdf) describes the IAU
model and phase conventions. Coefficient-rights and independent
complete-series reference fixtures still need formal review.
- The original MHB2000 planetary series uses **simplified lunar
  arguments** (not supplied by PR #52), and historical rounding/polynomial
  choices also differ; simply concatenating F1–F14 values from PRs #52
  and #53 does **not** reproduce the full source model.
- The connected draft PRs are **not yet on `dev/ephemeris`**. This
  module deliberately accepts caller-provided angles and rates so it
  stays independently compilable and testable without merging PR
  #52/#53 or importing third-party data.
- The prior draft PR #51 accepts externally supplied Δψ and Δε and
  calculates a mean→true-of-date rotation. Supplying that rotation
  with **TDB** derivatives from this API is not yet compatible with
  its **TT** derivative contract.

## CPAL-1.0 modification record

2026-11-10 — Added original Rust `src/nutation_harmonics.rs`,
`tests/nutation_harmonics.rs`, an export from `src/lib.rs`, and
this document/README reference. No official coefficients, third-party
program source, ephemeris binaries, new dependencies, external network
calls or production code were introduced. Existing CPAL-1.0 source
notices and mandatory **Powered by VirgoStars**
(https://www.virgostars.it/) attribution remain unchanged.
