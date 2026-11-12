# Observed CIP dX/dY and consistent CIO s reconstruction

**Date:** 2026-11-12. **Status:** draft, experimental, stacked on
unmerged observer [PR #120](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/120).
This implements **observational CIP corrections to a caller-provided
IAU model**, not the full IAU 2006/2000A precession/nutation series.

## Independent standards and meaning

The published [CelesTrak CSV EOP documentation](https://celestrak.org/SpaceData/EOP-format.php)
specifies `DX` and `DY` in **arcseconds**, distinct from terrestrial
polar motion `X,Y` in the same file. CelesTrak draws these celestial
pole offset data from IERS EOP C04 (IAU 2000A convention), with
observed and predicted flag `O/P`.

The official [IERS Bulletin B](https://datacenter.iers.org/products/18/1171/output/21300/bulletinb_IAU2000-216.txt.html)
describes this convention as:

```text
dX = X_observed - X_IAU2000A
dY = Y_observed - Y_IAU2000A
```

An application **must supply an independently evaluated nominal
IAU-compatible CIP model** `X_model(TT), Y_model(TT)` and the
CIO series quantity `q(TT) = s(TT) + X_model(TT)*Y_model(TT)/2`
from the same TT model/equinox/axes realization. It should validate
the nominal IAU convention against the IERS EOP field semantics.

After linearly interpolating the CSV's observed/predicted dX/dY
using the same UTC fraction and explicit quality policy already used
for terrestrial `xp/yp`:

```text
X_corr = X_model(TT) + dX_EOP
Y_corr = Y_model(TT) + dY_EOP
s_corr = q_model(TT) - X_corr*Y_corr/2
```

`X,Y` represent **dimensionless** CIP unit-vector components
(and have a natural small-angle equivalent in radians);
`dX,dY` are explicitly converted to radians from CSV
arcseconds; `s` and `q` are **radians**.

The series `s+XY/2` is a standard compact representation of the CIO
locator. This preserves the date-dependent series value
rather than erroneously using `s_model` unchanged after
correcting X/Y. The numerical correction does **not** implement
the IAU series and does not prove that external model values
were computed correctly.

## API

The existing `eop_polar_csv.rs` now exposes
`cip_dx_rad` and `cip_dy_rad` in
`DailyEopPolarMotion` and `InterpolatedEopPolarMotion`
while retaining the original chronological MJD/CSV, unit and
ObservedOnly/AllowPredicted guards.

The parser converts dX/dY via `π/(180*3600)` and deliberately rejects
values larger than 1 arcsecond in magnitude as an obvious-unit-error
guard (this is a software restriction, **not a formal IERS bound**).
Missing, NaN or infinite input is already rejected by strict CSV
validation. No source or dataset is downloaded automatically.

The new Rust `eop_cip_cio.rs` exposes:

```rust
use virgostars_ephemeris_core::eop_cip_cio::{
    ModeledCipCioAtTt, wgs84_gcrs_with_eop_corrected_iau_cip,
};
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_ut1_era::eop_ut1_era_from_utc_si_seconds;
use virgostars_ephemeris_core::observer_gcrs::Wgs84ObserverSite;

# fn example(
#   catalog: &CelestrakEopPolarCatalog,
#   site: Wgs84ObserverSite,
#   x: f64, y: f64, s: f64,
# ) -> Result<(), Box<dyn std::error::Error>> {
let mjd_utc = 51544;
let elapsed_utc_si_seconds = 0.0;
let policy = EopPredictionPolicy::ObservedOnly;

let tt_epoch = eop_ut1_era_from_utc_si_seconds(
    catalog, mjd_utc, elapsed_utc_si_seconds, policy
)?.tt_epoch;

// x,y,s MUST originate from an independently validated compatible
// IAU model evaluated at precisely this TT epoch; NEVER substitute
// the example epoch's historical ERFA or arbitrary numbers.
let modeled = ModeledCipCioAtTt {
    tt_epoch,
    model_x: x,
    model_y: y,
    cio_s_plus_xy_half_rad: s + x*y/2.0,
    source_label: "caller-provided-IAU-model-at-same-TT",
};

let result = wgs84_gcrs_with_eop_corrected_iau_cip(
    catalog, mjd_utc, elapsed_utc_si_seconds, policy,
    site, modeled
)?;
println!("corrected CIP/CIO {:?}", result.corrected_cip_cio);
println!("Earth-centered GCRS km {:?}", result.observer.observer_gcrs.position_gcrs_km);
# Ok(())
# }
```

The wrapper reuses the existing leap-second-aware
`eop_ut1_era_from_utc_si_seconds` (PR #116) and the
original `wgs84_gcrs_from_eop_utc_and_cip_cio` (PR #120),
so it does **not** reimplement UT1/TT, ERA, WGS84, EOP
or matrix frame rotations. It enforces exact **split TT**
epoch matching and nonempty model provenance and preserves
observed/predicted quality in the output. It also keeps both
original IAU model coordinates and EOP observational offsets
so a scientific review can inspect them.

A label does not authenticate either external dataset. All
outputs are **position-only GCRS**, not observer velocity,
planetary apparent positions or DE441 ICRF transformations.

## Numerical regression evidence

Two independent public numerical references are used:

1. **CelesTrak EOP published CSV row** `2000-01-01,MJD=51544`:
   dX = −0.000005 arcseconds and dY = −0.000050 arcseconds.
   These are now verified after conversion into radians,
   together with same-day exact-row behavior.
2. **ERFA published `t_xys06a` fixture**, at TT MJD 53736:
   `X=0.5791308482835292617e-3`,
   `Y=0.4020580099454020310e-4`,
   `s=-0.1220032294164579896e-7`.
   With explicitly **zero synthetic dX/dY**, the formula
   reconstructs the published s to a strict tolerance.
   That test is an algebraic identity of a published model output,
   **not** a claim that synthetic EOP inputs match that TT epoch.

The new parser regression groups verify published and interpolated
CIP offsets, forecasts and invalid units. The new end-to-end groups
verify 0h and half-day corrections, direct comparison against the
already-reviewed GCRS adapter, observed-vs-predicted status, rejected
model/TT mismatches, invalid model coordinates/label,
bad EOP columns, and missing time coverage.

## Remaining necessary work

**Full IAU 2006/2000A precession-nutation X/Y/s(TT) generation
remains unimplemented in this stacked observer branch.**
It is not acceptable to treat the external model inputs, a
published fixed-epoch ERFA test tuple, or zero offsets as a substitute.

The separate draft nutation/precession efforts (e.g. #48,
#54, #68) must be integrated and independently checked against
real-date SOFA/ERFA XYS or IERS values, with clear mean/true
frame conventions, before replacing manual inputs.

Additional science gates: verify current genuine EOP records
and leap-second schedule, measured dX/dY interpolation and
error budget, ITRS realization, observer/station velocity,
GCRS/BCRS reconciliation, DE441/NAIF and Horizons parity,
light-time and apparent-sky corrections. This PR's numeric
tests establish a **bounded piece of the arithmetic contract**,
not end-to-end observational accuracy.

## Licensing and workflow

The new source/test file uses the required original **CPAL-1.0
Exhibit A** license header, and VirgoStars visible attribution
remains mandatory. All implementation source is original;
independent numerical reference *values* and basic public
equations are used, not external astronomy source code.
No dependencies, binary kernels, new EOP datasets, remote
downloads, GitHub Actions workflow changes, releases,
deployments or main merges.

This branch is a scoped review PR **stacked on #120**; the
ancestor drafts are not modified or merged. Run authorized
read-only CI on a temporary PR targeting `dev/ephemeris`
if needed, then close without merge.

**References:** [CelesTrak EOP](https://celestrak.org/SpaceData/EOP-format.php);
[IERS celestial offsets](https://datacenter.iers.org/products/18/1171/output/21300/bulletinb_IAU2000-216.txt.html);
[ERFA test fixtures](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
