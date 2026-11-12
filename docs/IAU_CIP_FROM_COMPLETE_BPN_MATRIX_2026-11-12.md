# IAU 2006/2000A BPN matrix → CIP X/Y: a strict input adapter

**Date:** 2026-11-12. **Status:** experimental, draft stacked on
[PR #124](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/124),
which already has EOP dX/dY corrections and observer orientation.

## What this adds

The previous observer API accepts externally modeled CIP coordinates
`X,Y` plus a separate CIO series term `q=s+X*Y/2`, all at the
same explicitly tagged **TT** epoch as the EOP model. This scoped
change adds a mathematically independent way to obtain `X,Y`
**from an independently computed COMPLETE IAU 2006/2000A
bias-precession-nutation (BPN) matrix**.

For a GCRS→true equator/equinox-of-date right-handed matrix
`R_BPN`, the GCRS Cartesian unit direction of the celestial
intermediate pole is its **third row**:

```text
X = R_BPN[2][0]
Y = R_BPN[2][1]
Z = R_BPN[2][2] > 0
```

**Not the third column.** The extraction corresponds to the
published SOFA/ERFA `bpn2xy` convention.

The original pure-Rust adapter
`iau_cip_from_bpn::modeled_cip_cio_from_complete_bpn`
takes a `CompleteBpnAndCioAtTt` containing:
- exact **split-Julian-date** TT epoch (`TimeScale::Tt`);
- caller-supplied full `[[f64;3];3]` GCRS→true-equator
  **bias-precession-nutation** matrix (row-major, column vectors);
- independent CIO series `q(TT)=s(TT)+X*Y/2` in **radians**;
- nonblank, externally accountable model source label.

It returns the existing `ModeledCipCioAtTt` type from the
unmerged [PR #124](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/124).
That value can flow directly through
`wgs84_gcrs_with_eop_corrected_iau_cip` and the already-validated
EOP/TT/UT1/ERA/WGS84→GCRS pathway without duplicating mathematics.

## Why the model is **not** yet complete

A full standalone implementation must produce `R_BPN(TT)`
and `q(TT)` with the IAU 2006 precession and IAU 2000A nutation
series, including all required periodic terms and validated
source tables. **This PR does neither.**

A **mean-only precession** matrix is a valid orthogonal matrix but
is **NOT an acceptable complete BPN source**, even if it passes
orthogonality, determinant and range checks. A source label
cannot authenticate the input. CIO `q` cannot be deduced from the
BPN orientation alone. Supplying a constant, zero, or numerical
value for the wrong epoch would silently corrupt the physical
orientation if treated as actual model data; this adapter
therefore exposes rather than hides the external requirement.

The matrix adapter is a *small, audit-ready step* toward
integrating the ongoing [nutation/precession work](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pulls),
such as draft #48 and #68; it does not merge those branches,
duplicate their harmonic-series work or generate a truncated
substitute model.

## Usage (no real model is implicitly provided)

```rust
use virgostars_ephemeris_core::iau_cip_from_bpn::{
    CompleteBpnAndCioAtTt, modeled_cip_cio_from_complete_bpn,
};
use virgostars_ephemeris_core::eop_cip_cio::
    wgs84_gcrs_with_eop_corrected_iau_cip;
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopPredictionPolicy,
};
use virgostars_ephemeris_core::observer_gcrs::Wgs84ObserverSite;
use virgostars_ephemeris_core::calendar::JulianDate;

# fn example(
#   catalog: &CelestrakEopPolarCatalog,
#   utc_mjd: i64, utc_elapsed_si_seconds: f64,
#   site: Wgs84ObserverSite, tt: JulianDate,
#   bpn: [[f64;3];3], cio_series_q: f64,
# ) -> Result<(), Box<dyn std::error::Error>> {
let modeled = modeled_cip_cio_from_complete_bpn(CompleteBpnAndCioAtTt {
    tt_epoch: tt,                   // MUST match EOP-derived TT
    gcrs_to_true_equator: bpn,      // MUST be complete, at TT
    cio_series_q_rad: cio_series_q, // MUST be same-TT IAU series
    source_label: "externally-validated-IAU-2006-2000A-BPN-and-CIO",
})?;
let result = wgs84_gcrs_with_eop_corrected_iau_cip(
    catalog, utc_mjd, utc_elapsed_si_seconds,
    EopPredictionPolicy::ObservedOnly, site, modeled,
)?;
println!("{:?}", result.corrected_cip_cio);
# Ok(())
# }
```

## Input validation

Inputs must satisfy all these numerical rules:
- explicit `TimeScale::Tt`, representable split Julian day;
- finite elements in a 3×3 row-major matrix;
- orthogonality and unit-length rows within **5×10⁻¹¹**;
- right-handed determinant +1 within the same numerical tolerance;
- north-pole `Z>0`, with `X²+Y²<1`;
- finite CIO q in `[-π,+π]`;
- nonblank source label.

These are geometric and numerical contracts, **not science
authentication**. A transposed or mean-only matrix can be
orthogonal and right-handed: the caller is still responsible
for verifying the actual GCRS→true-equator convention.

## Independently published numerical tests

The new tests cite *reference numerical values* only, not source
code from SOFA/ERFA:

1. `t_pnm06a`: published full IAU 2006/2000A BPN matrix
   at TT JD `2400000.5 + 50123.9999`. The extracted X,Y
   must have the original independent third-row signs:
   `X=-0.3639337469629464969e-3`,
   `Y=-0.4163377605910663999e-4`.
2. `t_bpn2xy`: a second independent full 3×3 matrix gives
   `X=1.093465510215479e-3`,
   `Y=-4.281337229063151e-5`; the third-column
   alternative has the wrong sign and must not be used.
3. Identity and analytically rotated synthetic matrices verify
   the geometric projection and preservation of CIO `q`.
4. Invalid matrices (NaN, shears, dependent rows), left-handed
   reflections and south-facing orientations fail with typed
   errors.
5. UT1-tagged epochs, missing provenance, NaN/out-of-range
   CIO q values are refused.
6. A synthetic EOP example demonstrates that X/Y obtained
   from the BPN matrix flow through the existing EOP dX/dY
   correction and GCRS observer function. **Its ERFA matrix
   belongs to another physical epoch**: this verifies only
   software composition, not independent real-date IERS parity.

See [ERFA original test fixtures](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
No ERFA/SOFA implementation has been copied.

## Scientific gates not satisfied

Still missing are an independently verified complete
`IAU 2006/2000A BPN(TT)` generator, the `s+XY/2`
periodic series, consistent observed dX/dY IERS EOP
reference realization, independently vetted leap-second
schedule, and real-date SOFA/ERFA numerical comparisons for
the entire EOP→CIP/CIO→observer path. Real DE441/Horizons
astrometry, apparent positions and inertial observer velocities
are outside this scope.

The source/test files include the original CPAL-1.0 Exhibit A
notices and required VirgoStars credit obligations. No new
dependencies, copied astronomy code, datasets, kernels,
GitHub Actions workflow authoring, releases, deployment
or merge to `dev/ephemeris`/`main` is included.

**Sources:**
[IERS Conventions 2010](https://iers-conventions.obspm.fr/conventions_material.php);
[ERFA t_pnm06a/t_bpn2xy numerical fixtures](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
