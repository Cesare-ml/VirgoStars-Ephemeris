# Classic DE441 multi-series granule grid planner

**2026-11-12 · Experimental header-derived metadata planner, not an astronomical validation receipt.**

This change is a focused draft stacked on [PR #144](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/144), whose jump-aware piecewise longitude envelopes require genuine **individual polynomial validity intervals**. Unlike artificial single-rate or sample-only approximations, the new planner uses the *classic DE441 reader's validated header* to identify every relevant Chebyshev principal-record and per-series granule boundary in a requested TDB interval.

## Actual source integration

The existing `De441<R: Read+Seek>` now exposes:

```rust
pub fn granule_layout(&self) -> ClassicDe441GranuleLayout
```

This is a **read-only snapshot** of the header already checked by `De441::new()`, including `start_jd_tdb`, `end_jd_tdb`, `record_step_days`, `record_count` and 13 optional per-series granule counts in the classic JPL pointer order. Missing series remain `None`. No additional kernel binary copy, coefficient extraction, seek or second parser is introduced.

The new `de441_granule_plan` module provides:

```rust
pub fn plan_de441_relative_granules(
    layout: &ClassicDe441GranuleLayout,
    target: Body,
    observer: Body,
    start_jd_tdb: f64,
    end_jd_tdb: f64,
) -> Result<ClassicDe441GranulePlan, De441GranulePlanError>
```

Example in a real caller with an authorized classic DE441 file:

```rust
use virgostars_ephemeris_core::de441::{Body, De441};
use virgostars_ephemeris_core::de441_granule_plan::plan_de441_relative_granules;

let reader = De441::open("/external/path/de441.eph")?;
let layout = reader.granule_layout();
let plan = plan_de441_relative_granules(
    &layout, Body::MarsBarycenter, Body::Earth,
    2451545.0, 2451546.0,
)?;
for piece in &plan.slices {
    println!("TDB JD {}–{}: {:?}", piece.start_jd_tdb,
             piece.end_jd_tdb, piece.active_granules);
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

This sample **does not claim** the example kernel is present or that the given dates are covered by any particular supplied file. The planner reads metadata only; it does not evaluate a single position or velocity.

## The crucial Earth/EMB/Moon distinction

The classic DE441 reader already knows that `Earth` is not simply the Earth–Moon barycenter. A barycentric `Earth` state requires both:
- classic series index **2** (Earth–Moon barycenter), and
- classic series index **9** (geocentric Moon).

Other body mappings follow the current reader implementation: Mercury 0, Venus 1, Mars barycenter 3, Jupiter barycenter 4, Saturn barycenter 5, Uranus barycenter 6, Neptune barycenter 7, Pluto barycenter 8, Sun 10. Solar System barycenter itself needs no coefficient series.

For relative `target/observer` geometry the planner takes the sorted union of all **required** series. This may deliberately overpartition special Earth–Moon cases; overpartition is safer than missing a record boundary. If any required source series is absent, the function fails closed.

Each returned `ClassicDe441CoverageSlice` contains:
- a strictly increasing `start_jd_tdb` and `end_jd_tdb`;
- the exact `ActiveSeriesGranule` selections `{series_index, principal_record_index, subrecord_index}` for all required series at that slice's interior.

For every requested series, the grid is built using `record_start + subrecord_index * (record_step_days/granules)`, the same granule placement convention as the existing `De441::series()` evaluator. All principal-record and required subrecord boundaries are merged into one timeline. A piece never intentionally crosses a selected polynomial boundary.

## Rejection and precision rules

- Explicit finite TDB Julian-day interval, `start < end`, fully within the validated file coverage. No UTC conversion is performed.
- Invalid header geometry and absent required coefficient series fail.
- No same-body longitude (undefined relative direction).
- Limit requests to at most 512 principal records and 4096 output pieces; large historical ranges must be split at higher layers with separately verified coverage.
- Strictly increasing boundaries are required **within each per-series grid**; when two *different rational granule fractions* collapse to one rounded f64 Julian date, the planner rejects the request rather than silently deduplicating them. Sorting and deduplication are otherwise permitted for genuinely coincident fractions (e.g., 1/2 and 2/4). Each retained piece also requires a **strictly interior representable f64 midpoint**. If Julian-day resolution is insufficient, planning fails closed instead of claiming coverage.
- The results are metadata/selection indices only. **They do not assert physical continuity or a bounded jump across a principal record or subrecord transition.** Record position and derivative evaluation, independent endpoint errors and valid center chains remain future work.

## Validation performed / not performed

Ten **authored Rust unit tests** cover:
- Mars-barycenter vs Earth with combined EMB + geocentric-Moon grids (2 vs 4 subrecords);
- crossing a principal-record boundary;
- different granule counts including Sun;
- missing series and uncovered ranges;
- invalid interval and same-body input;
- invalid header count/radius;
- fractional requested windows;
- Solar-System-barycenter vs planetary series;
- granule boundary precision loss and collisions between distinct rational subrecord grids at extreme Julian dates.

**Separately executed** Python 3 reference stress test of the mathematical grid union: 11,911 random synthetic windows, totaling **4,450,734 sampled time-point/subrecord-index comparisons**, with **zero mismatches**. This is a check of the planner's design, not compilation/execution of the committed Rust code, not DE441 data validation, and not independent NASA/JPL positions. Separately, two crafted extreme-JD floating-precision cases reproduced collapsed per-series dates and 16 collisions between distinct rational fractions; the Rust tests explicitly require rejection in both cases.

The assistant environment lacks `cargo` and `rustc`. Therefore Rust compilation, native unit tests, fmt, Clippy and real scientific tests were **not run**.

Use an authenticated checkout to execute:

```sh
cargo fmt --all -- --check
cargo test --workspace de441_granule_plan
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Note that `de441.rs` is also modified by unrelated ongoing reader/test PRs; review downstream integrations for merge conflicts before any merge. Do not force-push or resolve file overlap by discarding other contributors' changes.

## Remaining scientific gates

1. For each planned series selection, extract the **actual validated Chebyshev coefficients** from the reader without duplicating parsing or exposing corrupt record data; account for Earth vs EMB and the geocentric Moon in one center-chain-aware solver.
2. Derive numeric rate/error bounds and boundary position/jump limits for each chosen piece, including source coefficients' representational error, frame derivatives and record transitions. A selected grid alone cannot prove a smooth path.
3. Connect the verified piecewise input to [PR #144](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/144) without blindly assigning `Some(0.0)` junction jumps. Any discontinuity budget must be justified with independently checked bounds.
4. Derive actual **apparent geocentric ecliptic-of-date** longitude with matched time scales/corrections and independently validate against official NASA/JPL/Horizons/NAIF data before satisfying the proposed [Astrological Functions PR #11](https://github.com/Cesare-ml/astrological-functions/pull/11) Rust/WASM provider contract.

No JPL kernel binaries, external code, dependencies, GitHub Actions changes, public package releases, production deployment, license edits or `main` merge. License and source notices remain **CPAL-1.0**, with VirgoStars attribution from LICENSE/NOTICE.
