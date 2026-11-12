# UTC elapsed SI seconds → EOP UT1−TAI → split UT1/TT → IAU ERA

**Date:** 2026-11-12. **Status:** original, experimental **draft**,
stacked on PR #112; no production time-scale service, no real
source-authenticated EOP or full ITRS→GCRS orientation solution.

## Purpose and source branches

The original EOP reader PR #109 (integrated in draft PR #112) provides
daily, **0h UTC** CelesTrak CSV `xp,yp`, **DUT1 = UT1−UTC** and
integer `DAT = TAI−UTC`. The previous PR #112 used this
information to form a TT epoch and the conventional **approximate**
SP00 TIO s′(TT)/ITRS→TIRS polar-motion matrix W, but **correctly
refused to assume an ordinary 86400-second UTC day across a leap
second**.

The existing IAU 2000 Earth Rotation Angle module (PR #75, integrated
on observer branch #89) accepts only a split **UT1** Julian date,
not a UTC date. This feature fills the missing rigorous **time
contract between daily EOP, UTC leap seconds, UT1, and ERA**.
The separately authored `earth_rotation`, `eop_polar_csv` and
`tio_polar_motion` modules are reused; none is reimplemented.

This stacked PR starts from the tested head of PR #112:
`fea379da221fcec92bb5767c4d78f9b87623df9b`. The component
PRs #84/#89/#100/#105/#109/#112 remain unchanged and unmerged.

## New Rust API

```rust
use virgostars_ephemeris_core::eop_polar_csv::{
    CelestrakEopPolarCatalog, EopPredictionPolicy,
};
use virgostars_ephemeris_core::eop_ut1_era::eop_ut1_era_from_utc_si_seconds;

# fn example(snapshot_csv: &str) -> Result<(), Box<dyn std::error::Error>> {
let catalog = CelestrakEopPolarCatalog::parse_csv(
    "Externally verified CelesTrak snapshot with date and legal provenance",
    snapshot_csv,
)?;
let model = eop_ut1_era_from_utc_si_seconds(
    &catalog,
    57753,    // UTC 2016-12-31 (positive leap second day)
    86400.0,  // UTC 23:59:60 exactly; extra SI second
    EopPredictionPolicy::ObservedOnly,
)?;
println!(
    "UT1={:?}, TT={:?}, ERA={} rad, W={:?}",
    model.ut1_epoch,
    model.tt_epoch,
    model.earth_rotation.angle_rad,
    model.approximate_polar.itrs_to_tirs
);
# Ok(())
# }
```

**This snippet is illustrative only:** a real file must contain
adjacent UTC daily rows including a DAT leap change, and the caller
must verify the source's authenticity, validity and permitted use.
`57753` is a genuine leap-second day, but **no EOP dataset is bundled**
and the example alone is not a verification of scientific results.

The mandatory `seconds_into_utc_day` parameter is a *physical elapsed
SI-second count since UTC 0h*, **NOT** a floating UTC Julian-day
fraction. Its valid range is half-open `[0,86400 + ΔDAT)`,
where `ΔDAT` is the change in the two consecutive, externally
supplied daily TAI−UTC offsets:

| Day class | ΔDAT | Valid elapsed SI seconds |
| --- | --- | --- |
| Ordinary UTC day | 0 | [0, 86400) |
| Positive leap second day | +1 | [0, 86401) |
| Negative leap second day (hypothetical test) | −1 | [0, 86399) |

An interval at `86400.0` on a positive leap day is the extra
`23:59:60` second. The *following* UTC `00:00:00` is on the next
MJD day at second `0.0`; no duplicate or missing physical SI
second is produced. Invalid infinity/NaN/negative/out-of-day
values fail closed.

**Date and data constraints:** post-1971 UTC only, exactly
consecutive daily EOP rows even for 0h queries, integer DAT
in a conservative numerical range 10–100 s, daily leap change
restricted to −1/0/+1 s, `|DUT1| <= 0.9s`, and at most
0.1s/day change in the *continuous* `UT1−TAI` reference
series. These numerical guards are **not authentication or
certification** of the actual IERS schedule, and an apparently
valid falsified CSV may still pass. No future leap-second
schedule is assumed.

Predicted EOP records (`DATA_TYPE=P`) require explicit
`EopPredictionPolicy::AllowPredicted`; quality
`EopPolarQuality::IncludesPrediction` remains visible.
Missing days are rejected; no silent interpolation across gaps,
extrapolation or last-known DAT fallback is allowed.

## Why UT1−TAI is interpolated instead of raw DUT1

`DUT1 = UT1−UTC` is *not continuous* at the end of a leap day:
when TAI−UTC jumps by +1 s at midnight, DUT1 jumps by nearly
+1 s as well even though Earth rotation remains smooth.

At 0h UTC daily EOP boundaries, set:

```text
u0 = DUT1_0 − DAT_0         # UT1−TAI at this day's midnight, seconds
u1 = DUT1_1 − DAT_1         # UT1−TAI at next day's midnight, seconds

L  = 86400 + (DAT_1−DAT_0) # actual SI seconds in civil day
f  = elapsed_SI_seconds/L  # relative physical day fraction in [0,1)

u(f) = u0 + f*(u1−u0)      # continuous UT1−TAI interpolation

UT1 = UTC_midnight + (elapsed_SI_seconds + DAT_0 + u(f))/86400
TT  = UTC_midnight + (elapsed_SI_seconds + DAT_0 + 32.184)/86400
```

The **UT1 and TT Julian dates** are continuous time-scale
coordinates expressed as an integer day number plus a fractional
day in uniform **86400 SI-second days**. Epoch arithmetic
carries the fraction across midnight explicitly; it does not
collapse a 2.4-million-day timestamp into a lossy single `f64`
before adding fractional seconds.

The EOP `xp/yp` linear interpolator is reused with the same
*actual* civil-day fraction `f`. The existing
`earth_rotation_angle(ut1_epoch)` then produces ERA and its
published constant IAU derivative in radians per **UT1 day**.
The existing opted-in
`itrs_to_tirs_polar_motion_with_approximate_sp00(tt,xp,yp)`
uses TT to compute the *conventional secular* TIO s′ and W.

The returned structured result records **both TT and UT1 epochs**,
the UTC day length, raw physical elapsed UTC seconds, EOP
provenance/quality, interpolated `UT1−TAI`, instantaneous
`UT1−UTC` *before the closing leap boundary*, interpolated
xp/yp in radians, Earth Rotation Angle in radians/rad per UT1
day, and approximate polar W as an ITRS→TIRS matrix.

**This does not automatically combine ERA and W with CIP/CIO**:
the remaining precession/nutation and intermediate celestial
orientation input is separate and must have a common physical epoch.
Neither a full observer velocity nor complete topocentric/apparent
planetary coordinates are implied.

## Independent numerical verification

The Rust tests include **two independent published ERFA values**:

1. `t_utcut1`:
   `eraUtcut1(2453750.5,0.892100694,0.3341)`
   gives UT1 JD second part `0.8921045608981481481`.
   Tests synthesize 2006-01-15/16 daily EOP with constant DUT1
   0.3341 s and DAT 33 s and verify the derived split UT1
   fraction to `1e-12`. This also produces continuous
   `UT1−TAI=-32.6659s`, as expected.
2. `t_era00`:
   `eraEra00(2400000.5,54388.0)` gives
   `0.4022837240028158102` rad.
   Synthetic 2007-10-15 EOP with zero DUT1 and unchanged DAT
   must reproduce this reference to `1e-12` rad.

The **2016-12-31 positive leap-second date** and the
`23:59:59 → 23:59:60 → 00:00:00` SI-second sequence are confirmed
in the independently published
[NIST leap-second table](https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds).
The tests use explicitly **synthetic** DUT1 endpoints
−0.4 s and +0.6 s, with the historically correct DAT
36→37 s. The resulting `UT1−TAI` is constant −36.4 s, so
successive elapsed UTC SI seconds are each one second apart in
UT1 **and** TT rather than producing a fictitious leap jump.
Another test uses a **synthetic hypothetical negative leap second**.

Additional deterministic groups verify composition with the
previous PR #112 time/polar module on ordinary days; predicted
data opt-in, malformed DAT and DUT1, implausible inter-day
UT1−TAI drift, missing adjacent days, negative/nonfinite
elapsed UTC seconds, pre-1972 behavior, and agreement of
ERA incremental angle with the IAU 2000 UT1 angular rate.

**Limits of these oracles:** they check elementary time-scale
arithmetic and ERA against published numerical fixtures, **not
real-date IERS EOP numerical parity, EOP provenance, CIP/CIO
orientation, JPL/NAIF/DE441 ephemeris accuracy, or complete
VirgoStars production readiness**. Real-dated IERS EOP and a
signed/externally trusted leap-second schedule must be integrated
and verified independently before high-accuracy use.

## Review, attribution and build

New module/test contain original VirgoStars source under
the full CPAL-1.0 Exhibit A header and preserve mandatory
“Powered by VirgoStars” project attribution in consumers.
No external astronomy software code, SPICE toolkits, datasets,
JPL kernels or copyrighted tables were bundled; published
ERFA/NIST **reference values only** are used.

No automatic EOP downloads, implicit leap-second updates,
third-party runtime dependencies, GitHub Actions authoring,
release, deployment or `main` merge. Existing authorized
read-only Rust CI should verify Rustfmt, workspace tests
(debug/release), strict Clippy and Rustdoc, and native Windows/
macOS compatibility. The feature branch/PR remains draft
stacked on #112 pending independent scientific review.

**Sources:**
[CelesTrak EOP CSV data format](https://celestrak.org/SpaceData/EOP-format.php),
[SOFA/ERFA numerical test suite](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c),
[NIST leap-second schedule and DUT1](https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds).
