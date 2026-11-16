# MVP-1 fixed-ICRF light-time + annual-aberration CSV exporter

**2026-11-12. Experimental, draft-only, no package or website integration.**

The native Rust example `crates/ephemeris-core/examples/aberrated_csv.rs` exports the *already implemented* fixed-axis six-field light-time and annual-aberration pipeline to plain CSV, using a separately supplied **classic DE441** binary. It does not implement or duplicate any astronomy or independently validate real JPL accuracy. It is a practical inspection tool for reproducible offline calculations and future reference comparisons.

## Run examples

At the repository root with a working Rust toolchain and an **actual locally supplied** classic `de441.eph` file:

```powershell
# Explicit instant in civil time, +01:00 offset. Uses the bounded approximate
# UTC -> TAI -> TT -> TDB conversion in the existing Rust core.
cargo run --quiet -p virgostars-ephemeris-core --example aberrated_csv -- "D:\Astro\de441.eph" "2026-11-10T13:30:00+01:00" Sun Moon Mercury Venus MarsBarycenter > stars.csv

# Match an independent Horizons reference by its actual TDB Julian date.
# No unannounced timezone, delta-T or leap-second conversion in this mode.
cargo run --quiet -p virgostars-ephemeris-core --example aberrated_csv -- "D:\Astro\de441.eph" "tdb:2461354.5" Moon Venus MarsBarycenter JupiterBarycenter > horizons-tdb-sample.csv
```

**Argument contract:** `aberrated_csv <classic-de441.eph> <YYYY-MM-DDTHH:MM:SSZ|±HH:MM | tdb:JD> [body ...]`. The initial instant is mandatory; `tdb:<number>` must be finite. A civil string must carry an explicit UTC offset or `Z`; daylight-saving time is not inferred from place names. The current civil conversion accepts UTC 1972-01-01 through 2027-06-27 under the bounded bulletin schedule and the truncated USNO TDB−TT approximation; there are no arbitrary-date precision guarantees. The same ISO instant with distinct offsets produces the same reception TDB.

Supported optional body names are **case-sensitive**: `Sun`, `Moon`, `Mercury`, `Venus`, `MarsBarycenter`, `JupiterBarycenter`, `SaturnBarycenter`, `UranusBarycenter`, `NeptuneBarycenter`, `PlutoBarycenter`. With **no** explicit body list, the CLI outputs all ten in this order. Duplicate body arguments are intentionally retained. `Body::Earth` is excluded because geocentric sightlines to Earth have undefined direction. This CLI does not imply that the physical *planetary centers* of Mars–Pluto are calculated; these are JPL **system barycenters**.

The program resolves the **entire requested batch before writing stdout** so a failed target, missing DE441 data, malformed instant or invalid body name does not silently emit a partial table. Errors go to stderr and return a nonzero exit status. The program does not use network calls, external kernel downloads, hidden global state, third-party CSV libraries, API keys, directories outside the user-supplied file, or implicit telemetry. Standard output is machine-readable UTF-8 CSV with no commentary mixed into the table.

## Exact output schema

```csv
target,center,reception_tdb_jd,emission_tdb_jd,fixed_tilt_rad,longitude_deg,latitude_deg,distance_au,longitude_speed_deg_per_tdb_day,latitude_speed_deg_per_tdb_day,distance_speed_au_per_tdb_day,light_time_seconds,light_time_rate,observer_acceleration_source,frame,corrections
```

- **Time:** reception and emission are both explicit **TDB Julian dates**; reception is shared across all rows. Emission is earlier than reception by the positive one-way light time. Printed JD is rounded to 12 decimal places; double-precision epochs at current dates are limited by the original `f64` TDB representation.
- **Center / body:** `Earth` geocenter; exact Rust `Body` variants, including barycenters, retained.
- **Axes:** `frame=fixed_ICRF_equatorial` with `fixed_tilt_rad=0`. These are fixed equatorial **not** mean/true ecliptic of date. Angular longitude is normalized into `[0,360)`; latitude is signed.
- **Units:** longitude/latitude in **degrees**, distance in **IAU-2012 AU**, signed longitude/latitude speed in degrees **per reception TDB day**, distance speed in AU per reception TDB day. Negative longitude rate is retained; no "retrograde" label is automatically inferred from equatorial longitude rate.
- **Physical corrections:** `LT_plus_annual_SR_aberration` means the existing iterated one-way reception light time and special-relativistic annual aberration. Analytic Chebyshev Earth acceleration is evaluated once for the whole batch and the original code computes each body's rate; `observer_acceleration_source=AnalyticDe441Chebyshev`. The `light_time_rate` is dimensionless **seconds per second**, as in the underlying Rust API.

The exporter deliberately **does not** rename these fixed-ICRF longitudes "zodiac signs" or "apparent ecliptic-of-date" and it does not add a moving-date frame, precession/nutation, gravitational solar deflection, atmospheric refraction, topocentric parallax, planetary physical-center models or Swiss `swe_calc_ut` compatibility. The absence of these corrections is part of the machine-readable `frame` and `corrections` contract, not just a footnote. Numeric output is not a direct comparison to Horizons `VEC_CORR=LT` or `LT+S` without explicitly transforming the frames and correction sets. A table generated for a real kernel is **not independent JPL scientific validation**.

## Testing and review

`cargo test --workspace` includes the new example's **three unit tests**, because `crates/ephemeris-core/Cargo.toml` explicitly declares `[[example]] ... test = true`.

- Rejects a time lacking an explicit offset, `tdb:NaN` and `tdb:Infinity`; accepts a valid explicit TDB or offset-based civil instant.
- Accepts exact system-barycenter names and rejects Earth as a sightline or an ambiguous physical outer planet center.
- Uses a **writer-only synthetic fixture** (no real planet values) to verify the precise 16-column CSV contract, stable row duplication, signed angular/radial speeds, acceleration provenance, axes and correction labels.

The scientific pipeline itself is separately covered by the previously implemented synthetic DE441 end-to-end tests in draft [#118](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/118). Optional **real-kernel** tests live in sibling draft [#122](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/122), and require an actual classic DE441 binary locally. No real-file test runs on CI.

The authorized **read-only CI** may validate formatting, workspace tests and strict Clippy on Linux, plus Windows and macOS native tests. Use a temporary CI-only draft targeting `dev/ephemeris` when the main review is stacked; **close it without merging** after confirming results. Do not publish packages, enable deployment, check in JPL files or change production.

## License and contribution provenance

**CPAL-1.0 modification record, 2026-11-12:** new original Rust example with the unmodified source Exhibit A notice, `Cargo.toml` example test registration and this document. No dependency, commercial ephemeris, JPL kernel data or third-party source code has been bundled. The source retains `LICENSE` and `NOTICE`; the applicable project attribution remains **Powered by VirgoStars** — https://www.virgostars.it/.

**Dependency chain:** based on draft #118, which is based on #114 → #107 with the separate #96 / #110 civil adapter bridge. This CSV review must remain **draft** and is not eligible for merge into `dev/ephemeris` or `main` until scientific dependencies are reviewed and an explicit integration decision is made.
