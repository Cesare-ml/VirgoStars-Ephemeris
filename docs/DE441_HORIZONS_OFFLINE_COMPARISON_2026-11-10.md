# Offline DE441 vector comparison against JPL Horizons

**Added:** 2026-11-10. **State:** harness implemented; **no real NASA/JPL parity result has been established**.

This procedure compares the Rust classic-binary DE441 reader to an **independent,
manually reviewed** Horizons state-vector export, with explicit metadata and
user-selected tolerances. The example is offline and makes no network requests.
It does not contain or download ephemeris files, Horizons vectors, or Swiss code.

## Prerequisites and scope

- Obtain a legitimate **classic binary DE441** file independently and verify
  its origin, byte size, coverage and SHA-256; do not commit it.
- Query Horizons for exactly the same ephemeris solution **DE441**, not a
  possibly different default solution or a DE440/DE440s kernel. Record the
  solution/version reported by Horizons. If this cannot be confirmed, record
  that as a blocker; do **not** mark the comparison scientifically validated.
- Use `EPHEM_TYPE=VECTORS`, `VEC_TABLE=2`, `VEC_CORR=NONE`,
  `REF_SYSTEM=ICRF`, `REF_PLANE=FRAME`, `TIME_TYPE=TDB`,
  `TLIST_TYPE=JD`, `OUT_UNITS=KM-D`, and preferably
  `CSV_FORMAT=YES`, `VEC_LABELS=NO`.
- Only compare matching **geometric** target-minus-center vectors:
  coordinate frame **equatorial ICRF**, Julian date **TDB**, position **km**,
  velocity **km/day**. There is **no** light-time, aberration, nutation,
  precession, topocentric or apparent correction. Use the correct body:
  planet-system barycenters must not be silently replaced with planet centers.
- This harness accepts one target/center pair per curated CSV reference file.
  The unsupported-format case fails instead of silently converting units,
  reinterpreting the time scale or inventing data.

Horizons API reference: https://ssd-api.jpl.nasa.gov/doc/horizons.html

## Curated input contract

Prepare a local UTF-8 CSV with the following required header. Use one row per
independently captured JD TDB vector; **do not treat this template as a valid
reference result**.

```text
# provider=JPL_HORIZONS
# ephemeris=DE441
# time_scale=TDB
# frame=ICRF
# plane=FRAME
# corrections=NONE
# units=KM-D
# target=Moon
# center=Earth
# query_url=<recorded exact Horizons API query>
# retrieved_utc=<actual UTC timestamp>
jd_tdb,x_km,y_km,z_km,vx_km_day,vy_km_day,vz_km_day
<JD_TDB>,<X>,<Y>,<Z>,<VX>,<VY>,<VZ>
```

Replace the angle-bracket placeholders with actual **numeric** values before
running. The raw Horizons CSV has additional columns such as a calendar date
and may have trailing delimiters: retain only the seven numeric columns in
the declared order. Preserve the original API response and exact query
separately for audit, together with both reference and kernel hashes. A label
such as `ephemeris=DE441` is **not** proof that Horizons used DE441: humans
must verify the JPL solution in the original response. The program validates
the metadata contract and finite numbers, not the cryptographic authenticity
of the reference or the provenance of a supplied file.

The `target` and `center` values must be exact supported `Body` variant
names (e.g. `Moon`, `Earth`, `Sun`, `SolarSystemBarycenter`,
`EarthMoonBarycenter`, `JupiterBarycenter`). Use `Moon` relative to
`Earth` for a straightforward first comparison. For other pairs, repeat the
workflow with a separate reference CSV and documented Horizons request.

## Running and interpreting results

```sh
cargo test --workspace --example validate_horizons --locked
cargo run --example validate_horizons -- \
  /path/to/legally-obtained/de441.eph \
  /path/to/reviewed/horizons-reference.csv \
  <approved_max_component_position_error_km> \
  <approved_max_component_velocity_error_km_per_day>
```

The final two arguments **must be positive finite numbers**, chosen and
approved in the scientific validation plan. There are deliberately no default
accuracy thresholds. For every epoch the example prints all **six signed
component residuals**, defined strictly as **Rust DE441 minus Horizons**:
`dx, dy, dz` in km and `dvx, dvy, dvz` in km/day. It also prints the
maximum absolute component error and Euclidean (3D) norm for position and
velocity, followed by overall maxima and the number of failing epochs.

The **pass/fail decision still uses the two explicitly approved maximum
absolute component tolerances**; Euclidean norms are reported as additional
diagnostics, not silently substituted as acceptance thresholds. Overflowing
or non-finite residual components/norms are rejected rather than counted as
passing. Any exceeded threshold, out-of-coverage JD, missing DE441 series,
invalid reference contract or unreadable file produces a nonzero exit code.

The parser has synthetic unit tests rejecting mismatched frame, plane,
corrections, units, unknown bodies, missing provenance and non-finite values.
A green GitHub Actions CI job only establishes those harness/parser checks.
**No real-kernel validation is run in CI**: the private/local reference
file and external DE441 binary are required to establish real scientific
agreement.


## Incremental validation improvement — 2026-11-10

The example now calculates signed XYZ/VXYZ residuals, maximum per-axis absolute
errors and 3D residual norms, with deterministic regression tests for signs,
units and large-magnitude numeric failures. These extra diagnostics support a
scientific audit of each body and epoch once real data is supplied; they do
**not** constitute an independent numerical comparison by themselves.

## Evidence required before claiming a passing scientific gate

Record the exact commit SHA and toolchain, binary file name, original source,
file size, SHA-256 and coverage, Horizons query/response including DE solution
identifier, timestamp, time/center/frame/correction options, approved
tolerances, compared cases, observed max errors and exceptions. Preserve
the originals according to their separate data terms. If the compared
datasets are different JPL solution versions, the result is not a strict
DE441 parity test.

## CPAL-1.0 modification record

2026-11-10 — Added
`crates/ephemeris-core/examples/validate_horizons.rs` (strict offline
reference validator and synthetic parser tests), updated
`.github/workflows/rust-dev.yml` (test the example), and created this
validation procedure. These changes derive from the original
VirgoStars-Ephemeris work by Initial Developer **Cesare-ml (VirgoStars
project)**. The executable source has the existing CPAL-1.0 Exhibit A notice.
No changes to the `LICENSE`, `NOTICE`, required **Powered by VirgoStars**
attribution (https://www.virgostars.it/), astronomical computation,
external dependencies or dataset redistribution.
