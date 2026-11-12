# EOP/UTC + explicit same-TT CIP/CIO → WGS84 ground observer in GCRS

**Date:** 2026-11-12. **Status:** experimental, position-only,
draft **stacked on PR #116** (and on its unmerged ancestors #112,
#105, #89). Not merged to `dev/ephemeris` or `main`; no release
or production migration.

## Scope and motivation

The recent original EOP stack already has:

1. Offline caller-supplied **CelesTrak 0h UTC daily CSV**, including
   observed-vs-predicted `xp/yp` in radians, `UT1−UTC`, `DAT=TAI−UTC`.
2. Leap-aware UTC SI-second processing, interpolated **UT1−TAI**
   (rather than discontinuous DUT1), explicit split **UT1 and TT**
   Julian dates, and IAU ERA(UT1). See PR #116.
3. The **conventional approximate** terrestrial TIO locator **s′(TT)**
   and ITRS→TIRS polar matrix W, in PR #105.
4. A separate original WGS84 geodetic site → ITRS → TIRS →
   CIRS → **GCRS Cartesian ground observer position in km**,
   in PR #89. This requires user-supplied CIP X,Y and celestial
   CIO locator s for the correct TT epoch.

This feature adds the narrow missing *application boundary* between
these established primitives: one quality-aware call that passes
the computed UT1/TT/EOP values to the position pipeline **without
making up a CIP/CIO model** or mixing their time scales.

## New Rust API

```rust
use virgostars_ephemeris_core::eop_observer_gcrs::{
    wgs84_gcrs_from_eop_utc_and_cip_cio, ExternalCipCioAtTt,
};
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_ut1_era::eop_ut1_era_from_utc_si_seconds;
use virgostars_ephemeris_core::observer_gcrs::Wgs84ObserverSite;

# fn example(
#   catalog: &CelestrakEopPolarCatalog,
#   site: Wgs84ObserverSite,
#   model_cip_x: f64, model_cip_y: f64, model_cio_s_rad: f64,
# ) -> Result<(), Box<dyn std::error::Error>> {
let utc_mjd = 54388;
let elapsed_utc_si_seconds = 0.0;
let policy = EopPredictionPolicy::ObservedOnly;

// 1. Derive a TT epoch from the *specific* external EOP snapshot.
let epoch = eop_ut1_era_from_utc_si_seconds(
    catalog, utc_mjd, elapsed_utc_si_seconds, policy,
)?.tt_epoch;

// 2. Independently compute X,Y,s from a vetted physical CIP/CIO
// model at EXACTLY that TT split JD, in matching GCRS axes.
let external = ExternalCipCioAtTt {
    tt_epoch: epoch,
    cip_x: model_cip_x,
    cip_y: model_cip_y,
    cio_locator_s_rad: model_cio_s_rad,
    source_label: "independently-validated-IAU-CIO-model",
};

// 3. Position-only Earth-centered GCRS Cartesian site.
let result = wgs84_gcrs_from_eop_utc_and_cip_cio(
    catalog, utc_mjd, elapsed_utc_si_seconds, policy, site,
    external,
)?;
println!("ITRS km: {:?}", result.observer_gcrs.position_itrs_km);
println!("GCRS km: {:?}", result.observer_gcrs.position_gcrs_km);
# Ok(())
# }
```

The sample's `model_cip_x`, `model_cip_y` and
`model_cio_s_rad` **must not be invented or filled with
the independently published numbers for the wrong epoch**.
They are intentionally absent from the library. A label is for
provenance auditing, NOT validation of its claimed model.

`ExternalCipCioAtTt` requires:

- `tt_epoch: JulianDate` explicitly tagged `TimeScale::Tt`,
  matching **exactly** the EOP-derived split TT day/fraction.
  Mismatched epochs (even microseconds) fail closed; this adapter
  does not resample or propagate the celestial pole/CIO model.
- `cip_x`, `cip_y`: dimensionless **GCRS Cartesian components
  of the celestial intermediate pole**, not longitude/latitude
  radians or EOP polar `xp,yp`.
- `cio_locator_s_rad`: **celestial** CIO locator **s**
  in radians, NOT terrestrial TIO locator **s′**.
- `source_label`: nonblank, describing independently obtained
  CIP/CIO data or an evaluated IAU model; cannot prove authenticity.

The returned `EopWgs84GcrsObserver` retains:

- the entire original quality/provenance-tagged
  `EopUt1EraOrientation` (UT1/TT, leap day length,
  ERA and its rate, xp/yp, conventional approximate s′ and
  polar motion W), including predicted/observed status;
- the CIP/CIO source label separately from the EOP source label;
- `ObserverGcrsPosition` with geocentric WGS84/ITRS and
  GCRS positions in **kilometers**, plus ERA angle in radians.

## Validation / independent numerical oracles

Eight new tests (original `tests/eop_observer_gcrs.rs`) cover:

1. **Independent published SOFA/ERFA `t_gd2gc`** WGS84
   lon=3.1, lat=-0.5 rad, ellipsoidal height 2500 m:
   ECEF/ITRS `[-5599000.5577049947, 233011.67223479203,
   -3040909.4706983363]` meters, compared after km conversion.
   **Independent `t_era00`** epoch `JD_UT1=2400000.5+54388.0`
   → `ERA=0.4022837240028158102` rad.
   These validate elementary position and rotation, not the
   chosen synthetic CIP/CIO model.
2. Complete observer output compared to the existing independent
   module `wgs84_observer_to_gcrs` at the same epoch/parameters.
3. Reject a CIP epoch incorrectly tagged UT1 and a distinct
   TT epoch even if only a few microseconds off.
4. Reject absent or whitespace-only CIP/CIO model provenance.
5. Propagate invalid CIP X/s values and invalid geodetic site.
6. On **2016-12-31 23:59:60 UTC** and the following
   midnight, confirm 1-second-continuous UT1 and a correspondingly
   small observer displacement, without mislabeling UT1 as UTC.
   EOP DUT1 values are synthetic; DAT 36→37 is historical.
7. Preserve explicit `EopPredictionPolicy::AllowPredicted`
   decisions and returned `IncludesPrediction` quality.
8. Propagate missing adjacent EOP records and invalid UTC
   elapsed SI seconds, without fallback.

The earlier branch #89's positional integration already tests
independent published SOFA/ERFA C2TCIO matrix and WGS84 fixtures
at an illustrative epoch, but **those CIP/CIO values are not
asserted to be physically time-matched to the synthetic EOP
snapshot in these new tests**. The present tests use synthetic
X,Y,s to exercise matrix composition and preserve the strict
time/provenance boundary. They are not independent real-date
IERS/CIP physical validation.

## Scientific limitations / next gates

- No automatic loading/interpolation of IERS EOP, leap-second
  tables, CIP X/Y or celestial CIO s.
- EOP input may be a forged/outdated CSV or prediction. The
  routine checks internal constraints and visible quality, not
  authenticity, observed frame realization or measurement error.
- Does NOT compute CIP/CIO from TT (IAU 2000A/2006 series), nor
  automatically apply dX/dY EOP celestial pole offsets.
- Only **position** is transformed. Derivatives of W(t), ERA and
  CIO matrix C(t), station plate motion, solid Earth tides and
  observer inertial velocity require independent implementations
  and scientific validation.
- GCRS Earth-centered position is **not** a raw DE441 BCRS/ICRF
  barycentric position. A physically validated GCRS↔DE441
  frame/epoch/origin mapping is required before topocentric
  planetary use (separate draft #97). No light-time,
  aberration, gravitational deflection, refraction, apparent
  ephemeris, Swiss parity or production precision is claimed.
- Full validation requires real-date independently traceable
  IERS/SOFA/ERFA observer/CIP/ERA numerical references,
  published error budgets, time-quality flags and later
  JPL/Horizons ephemeris comparisons with exact conventions.

## License and CI

New source/tests use the full project's CPAL-1.0 Exhibit A
notice; preserve required VirgoStars attribution; no external
source code copied, no EOP datasets, kernels, downloads,
dependencies, GitHub Actions authoring, releases or deploys.
The separate original PRs are not rewritten or merged here.

The exact new scope is **4 files**: one source module, one test,
one documentation file and a single public module export. This
draft is stacked onto #116, not a replacement for its reviewers'
approval. Repository-approved **read-only CI** is used for
Rustfmt, workspace tests (debug/release), strict Clippy, strict
Rustdoc and native Windows/macOS compatibility.

**References:**
[ERFA numerical oracle tests](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c),
[IERS Conventions 2010, chapter 5](https://iers-conventions.obspm.fr/conventions_material.php),
[CelesTrak EOP CSV source format](https://celestrak.org/SpaceData/EOP-format.php).
