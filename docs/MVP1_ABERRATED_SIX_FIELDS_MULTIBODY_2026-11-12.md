# MVP-1: multi-body geocentric six-field states after annual aberration

**2026-11-12 | Experimental integration, stacked on unmerged draft PR #77.**

## Motivation and architecture

The existing Rust pipeline in draft [#77](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/77) calculates one Earth's-center relative target at an **explicit TDB Julian date**, applying the current classic-DE441 reader, iterated reception light time, relativistic annual aberration with **analytic DE441 Earth acceleration**, and the existing spherical/rate projection and degrees/AU conversion. A chart or transit client must request several bodies at the same epoch and preserve their identity.

This change adds a **small compositional batch** to that exact stack; it is **not another astronomical algorithm**.

```rust
use virgostars_ephemeris_core::aberrated_spherical::geocentric_aberrated_fixed_six_fields_many;
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut de441 = De441::open("/path/to/your/classic-de441.eph")?;
let result = geocentric_aberrated_fixed_six_fields_many(
    &mut de441,
    &[Body::Sun, Body::Moon, Body::Mercury, Body::Venus],
    JulianDateTdb(2_451_545.0),
    0.0, // fixed ICRF equatorial axes
)?;
for body in &result {
    println!(
        "{:?}: longitude={} deg, speed={} deg/TDB day",
        body.fields.target,
        body.fields.longitude_deg,
        body.fields.longitude_speed_deg_per_tdb_day
    );
}
# Ok(())
# }
```

### Rust API

`aberrated_spherical::geocentric_aberrated_fixed_six_fields_many<R: Read + Seek>(&mut De441<R>, &[Body], JulianDateTdb, fixed_tilt_rad) -> Result<Vec<AberratedFixedSixFieldsDegreesAu>, AberratedSphericalError>`.

- **Time:** input is an *already-TDB* Julian date; this module never guesses UTC, timezone or TT. The bounded approximate civil→TDB conversion in draft #96 is **separate** and is not imported into the #77 stack. It can be connected only after both dependency lines have been reviewed.
- **Targets:** exact `Body` IDs retained; outer planet names are **system barycenters**, not physical planetary centers. Center is `Body::Earth`.
- **Frame:** fixed ICRF equatorial axes at `fixed_tilt_rad=0.0`, or caller-chosen **constant X tilt**, not the true/mean moving ecliptic of date.
- **Corrections:** existing reception light time and **annual aberration** with Earth's analytically differentiated classic-DE441 Chebyshev observer acceleration. No new or duplicate Lorentz boost, derivative, light-time solver or spherical conversion. In particular, `ObserverAccelerationSource::AnalyticDe441Chebyshev` is retained per result.
- **Units:** six consumer fields per target — longitude and latitude in **degrees**, distance in exact IAU-2012 **AU**, signed longitude and latitude rates in **degrees/reception TDB day**, radial speed in **AU/reception TDB day**. Each result also includes the emission TDB Julian epoch, light-time seconds and time derivative, as in the original scalar API.
- **Ordering and errors:** preserves body order and duplicates. Fails the *entire* batch on any invalid target, missing series, out-of-coverage **emission** epoch, undefined direction, or unit overflow. No partial `Vec` is returned; the original `AberratedSphericalError` subtype is preserved.
- **Empty batch:** validates finite epoch and finite tilt, then returns `Vec::new()` without accessing a DE441 coefficient record or validating that date against file coverage. This is **not** a promise that a target is available for the same epoch.
- **Performance:** uses existing scalar implementation for every target with the original `De441` reader cache. It does **not** yet share one Earth barycentric acceleration calculation across targets or promise a measured speedup; the purpose of this stage is a consistent multi-body **contract** and reuse of verified logic.

## Tests

The existing `crates/ephemeris-core/tests/de441_analytic_acceleration.rs` reuses its own fictional quadratic classic-DE441 records and includes four new regressions:

1. A Moon/Mercury/Moon input retains **order, duplicates, Earth center, reception/emission epochs, analytic acceleration source**, and equality of *all six fields* and metadata with independently requested scalar results.
2. The six-field batch at TDB `t±0.01` days is differentiated in **consumer units**; the signed circular longitude difference and latitude/radial differences must match the three rates returned by the middle full-pipeline call.
3. A missing Sun series after successful Mercury evaluation rejects the **whole batch** with the nested original error; a subsequent Mercury/Moon request works on the same reader. Earth-as-target is still an explicitly undefined geocentric sightline.
4. Empty target arrays succeed without target queries but still reject nonfinite TDB epochs and tilts.

Commands with stable Rust installed:

```bash
cargo fmt --all -- --check
cargo test --workspace --test de441_analytic_acceleration
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**Scientific limitations:** the polynomial coefficients are mathematically invented fixtures. Passing these tests **cannot** certify independent DE441/NAIF/Horizons agreement or true apparent longitude parity against Swiss Ephemeris. Missing physical effects include solar gravitational deflection/Shapiro delay, complete IAU precession/nutation, moving date-frame rate `Rdot*r`, observer topocentric parallax and some planetary-center models. The code does not yet deliver a Node/WASM/Python binding or a production replacement for `swe_calc_ut`. Only **classic DE441 binaries**, not SPK or Swiss `.se1`, can be supplied.

## Integration and provenance

**Dependency chain:** this draft branches from **#77**, itself stacked on **#76 → #73 → #46**. The separate civil-time development chain **#96 → #101 → #103** is untouched. Review the scientific correctness and merge dependencies in their own order; do **not** merge this PR alone into `dev/ephemeris` or `main`. The public scalar API remains identical.

**CPAL-1.0 modification record, 2026-11-12:** extended existing CPAL-covered `src/aberrated_spherical.rs` with a batch wrapper and `tests/de441_analytic_acceleration.rs` with four original synthetic tests, plus this English documentation file. Existing Exhibit A source headers and `LICENSE`/`NOTICE` are unchanged. There are no new dependencies, bundled JPL/NAIF/Swiss data/source, privilege escalations, automatic downloads, packages, releases or deployments. Applicable attribution remains **Powered by VirgoStars** — https://www.virgostars.it/.
