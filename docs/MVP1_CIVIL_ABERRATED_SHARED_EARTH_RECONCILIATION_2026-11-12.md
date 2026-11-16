# MVP-1 integration: civil multi-body output reusing one Earth state

**Date:** 2026-11-12  
**Status:** Experimental; separate unmerged draft review, no deployment.

## Scope and reasons for this reconciliation

Two already independently tested but **unmerged** development lines branch from a shared base, draft #107:

- [Draft PR #110](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/110) combines **civil time with explicit UTC offset → approximate TDB JD → geocentric multi-body fixed-frame six fields** after one-way light-time and annual aberration. It contains a byte-identical public time adapter from [draft #96](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/96).
- [Draft PR #114](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/114) makes the same multi-body calculation evaluate Earth's barycentric position, velocity and analytic Chebyshev acceleration **once per nonempty request**, sharing that state across targets while keeping the original scalar APIs and physics.

**This PR is based on the exact #114 head**. The original #110 branch is **not** modified or merged. The new branch carries **the same original #110 source module, civil-time delegate, existing tests and documentation**, plus two new integration regressions. This deliberate temporary overlap is documented to make the dependency conflict visible rather than silently merge unfinished branches. **When #96, #110, and #114 are eventually reviewed/integrated, the duplicate file additions must be reconciled**, not applied twice.

The resulting path:

```text
CivilTimestamp (Gregorian ISO-like with explicit UTC offset)
  -> [#96] to_approximate_tdb_julian_date(): UTC→TAI→TT→approximate TDB once
  -> [#110] civil_aberrated::geocentric_aberrated_fixed_six_fields_many_from_civil
  -> [#114] geocentric_aberrated_fixed_six_fields_many
         Earth/SSB position, velocity, analytic acceleration once at reception TDB
         per target: original DE441 iterated light-time / annual aberration
                     / projection / degrees-AU conversion, no duplicated model
  -> CivilAberratedSixFieldsBatch { reception_epoch, results }
```

## API contract

The **public signature is unchanged** from #110:

```rust
use virgostars_ephemeris_core::civil_aberrated::geocentric_aberrated_fixed_six_fields_many_from_civil;
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::first_sky::CivilTimestamp;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut eph = De441::open("/user-provided/classic-de441.eph")?;
let instant = CivilTimestamp::parse("2026-11-10T13:30:00+01:00")?;
let chart = geocentric_aberrated_fixed_six_fields_many_from_civil(
    &mut eph,
    &[Body::Sun, Body::Moon, Body::Mercury, Body::Venus],
    instant,
    0.0,
)?;
for item in chart.results {
    println!(
        "{:?}: longitude={} degrees, rate={} degrees/reception TDB day",
        item.fields.target,
        item.fields.longitude_deg,
        item.fields.longitude_speed_deg_per_tdb_day,
    );
}
# Ok(())
# }
```

The output uses:
- Epoch: **approximate TDB Julian date** derived from the caller's explicit UTC offset, limited by the bounded IERS Bulletin C 72 UTC/TAI table (1972-01-01 through 2027-06-27 inclusive) and a **truncated USNO TDB−TT approximation**. Single-`f64` JD limits subsecond precision.
- Target: exact `Body` identifiers, including **system barycenter** variants (not physical centers of all planets), relative to Earth's **geocenter**.
- Frame: **fixed ICRF equatorial** axes at `fixed_tilt_rad=0`, or a caller-provided **constant X tilt**; NOT mean/true ecliptic of date, with no `dR/dt` frame-velocity correction.
- Corrections: **one-way reception light-time + Earth's annual aberration** including analytically differentiated DE441 Earth acceleration. Missing solar deflection/Shapiro delay, complete precession/nutation, topocentric effects, date-frame derivatives and official IERS EOP processing.
- Units: longitude/latitude **degrees**, distance **IAU 2012 AU**; all three signed speeds in **degrees or AU per reception TDB day**. Each body retains emission TDB epoch, light-time seconds/rate, Earth center, exact target ID and analytic acceleration provenance.
- Semantics: preserves target order and duplicates; **all-or-error**, no partial results if a body fails. Empty batch validates civil time and finite tilt without asserting body coverage. Errors maintain typed `CivilTime(SkyError)` versus `Aberrated(AberratedSphericalError)` provenance.

The optimization avoids redundant Earth interpolation but **no measured overall speedup is claimed**; per-target light-time and boost computations remain. No network calls, third-party program source, external kernel downloads, new crate, Node/WASM/Python bindings, or package release.

## New integration tests and current scientific limits

The **existing four #110** tests are preserved verbatim on this branch. They run an invented in-memory classic-DE441 record through the full Rust reader, Earth/Moon reconstruction, observer acceleration, light-time and aberrated six fields; they prove software composition, **not** real JPL accuracy.

Two additional regressions, in `crates/ephemeris-core/tests/civil_aberrated_batch.rs`:

1. At UTC epochs 2026-11-03/10/20/30 with two constant tilts (0 and π/6), calculate the full civil-input **optimized batch** for `[Moon, Mercury, Moon]`. For **every** target, compare the **entire result exactly** (all six values and metadata) with an independent scalar `geocentric_aberrated_fixed_six_fields` call from a **fresh, separate** synthetic-DE441 reader; confirm TDB epoch, Earth center, order, duplicates, signs/finite speeds and analytic acceleration source.
2. For an Earth-as-geocentric-target after a successful Moon evaluation, verify **whole-batch failure** with the original typed `LightTimeError::UndefinedDirection`; verify reader remains reusable and an otherwise supported civil date **outside the fictional DE441 coverage** returns the original typed `De441Error::OutsideCoverage`.

These exact comparisons are regression checks against the same Rust astronomical model, **not independent scientific validation**. Before substituting for Swiss Ephemeris, compare real official JPL/Horizons reference vectors with aligned definitions, epochs, body centers, corrections, frames and tolerances. The current core supports only caller-supplied **classic DE441** binary, not `.bsp` SPK or Swiss `.se1` kernel formats.

CI plan (GitHub pre-approved **read-only** Rust checks; no local compiler claimed):

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run via a **temporary CI-only draft** with this exact head into `dev/ephemeris`, since the repository CI is configured for that PR target, then close the temporary PR **without merge** when complete. Linux, Windows and macOS results and any fixture limitations belong in the scientific PR review.

## Licensing, source provenance, safe integration

**2026-11-12 CPAL-1.0 modification record:** the #114-based branch adds the **source-identical** #110 `src/civil_aberrated.rs`, the exact 19-line #96 civil time public delegate in `src/first_sky.rs`, one `lib.rs` module export, the original #110 four-test file with **two further tests**, and the existing #110 English documentation plus this reconciliation document. The underlying #114 optimized Rust calculation files are **untouched**. CPAL-1.0 Exhibit A source notices remain intact; `LICENSE` and `NOTICE` are unchanged; no third-party ephemeris datasets or incompatible source code are bundled.

Attribution required by the applicable CPAL conditions remains **Powered by VirgoStars** — https://www.virgostars.it/.

**Dependency order:** #96 + upstream aberration #46 → #73 → #76 → #77 → #107 → #114, then reconcile the #110 civil adapter with this integration branch. **No merge into `main` or `dev/ephemeris` without explicit owner authorization, no release or production migration.** A passing synthetic CI is a code-quality milestone, not astronomical accuracy certification.
