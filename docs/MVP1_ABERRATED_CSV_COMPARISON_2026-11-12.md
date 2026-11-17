# Offline comparison of fixed-ICRF six-field CSV results

**2026-11-12 | Experimental scientific data-quality workflow, not an accuracy certification.**

## Why this tool exists

The `aberrated_csv` exporter from [draft #126](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/126) can produce deterministic, clearly labelled astronomical quantities. A direct subtraction against an arbitrary Horizons or Swiss table would be scientifically **incorrect** if the target, center, epoch, correction model or coordinate frame differs.

The new Rust example `crates/ephemeris-core/examples/compare_aberrated_csv.rs` therefore implements a **strict like-for-like gate** before calculating any residuals. It contains **no astronomical interpolation or correction formulas** and uses only the Rust standard library. It does not download external reference files, interpret different astrometric conventions or silently transform a mismatched input. It is a reproducible *consistency* and comparison tool, not a second ephemeris engine.

## Run it

Generate `actual.csv` using the experimental exporter on a separately obtained **classic DE441** file, or supply two existing CSV files that follow the exact exporter schema. Then, from the repository root:

```powershell
cargo run --quiet -p virgostars-ephemeris-core --example compare_aberrated_csv -- `
  actual.csv reference.csv `
  0.5 1e-8 1 1e-8 1e-6 1e-8 > residuals.csv
```

The six mandatory **nonnegative finite** thresholds are, in order:

1. Maximum **longitude or latitude position** error, arcseconds.
2. Maximum **distance** error, AU.
3. Maximum **longitude or latitude speed** error, arcseconds per **reception TDB day**.
4. Maximum **radial speed** error, AU per reception TDB day.
5. Maximum **light-time** error, seconds.
6. Maximum **light-time rate** error, dimensionless seconds/second.

These numbers are **illustrative user-selected tolerances only**, not experimentally validated accuracy targets. A zero tolerance requests exact equality, which is useful to compare identical exporter runs. The example prints a **complete error CSV report** if fields exceed the chosen tolerances, and then exits nonzero with an explanation on stderr. A structural input mismatch fails **before any report** is written to stdout; this prevents accidental partial scientific comparisons.

The tool's report contains the original target name, absolute longitude/latitude/distance errors, the three signed-speed **absolute residuals**, light-time and light-time-rate errors, and a per-target boolean indicating whether every user-specified tolerance passed. Longitude position errors are compared using a shortest-arc circular difference, so `359.99°` versus `0.01°` is **72 arcseconds**, not `~1.3 million arcseconds`. Signed speeds are not wrapped: e.g. `-1.0` versus `+1.0` degrees/TDB day differs by 7200 arcseconds/TDB day.

## Rejection and provenance gates

The input CSV must match the **exact 16-column** `aberrated_csv` header, with no extraneous columns/rows, unsupported target IDs, unknown correction modes or nonfinite values. Every row must have:

- A target from the exported ten-body set. `MarsBarycenter`, `JupiterBarycenter`, etc. are **system barycenters**, never silently a physical planet center. No Earth-as-Earth target.
- `center=Earth`, `frame=fixed_ICRF_equatorial` and `fixed_tilt_rad=0`; no equinox/ecliptic-of-date substitution.
- `corrections=LT_plus_annual_SR_aberration` and `observer_acceleration_source=AnalyticDe441Chebyshev`. A raw geometric, LT-only, Horizons LT+S, topocentric or Swiss apparent-of-date row is **rejected**, not adjusted.
- Finite Julian TDB reception and emission dates, strictly positive range and one-way light time, emission before reception, longitude in `[0,360)`, latitude in `[-90,90]`, and finite signed speeds/rates.
- Exactly one **shared reception TDB JD** for all rows within each file. The two files must have the **identical parsed reception TDB JD**, identical tilt, exact body order and target multiplicities; row counts must match. No nearest-neighbor matching or implicit timezone conversion. Duplicate bodies are supported and compared positionally only.

The comparison assumes that the reference producer has already **properly aligned** any external source to the specified frame, correction model and units. This tool validates labels and values but **cannot prove that an independent provider really computed those conventions**; a manually falsified label is not independent scientific evidence.

For any real independent-validation report, **retain separately** the original provider, original query/response, retrieval UTC date, kernel/ephemeris version and file SHA256, exact target identifiers, original time scale and frame, reference transformation steps, accepted tolerances and test-machine evidence. Do **not** treat a duplicate VirgoStars output file or a reference relabelled from Horizons `VEC_CORR=LT` or `LT+S` as independent agreement with a special-relativistic annual-aberration model. The [optional real classic-DE441 reference tests](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/122) check a different, explicitly described **recovered LT geometry** and do not establish full apparent parity.

## Deterministic unit-test scope

`Cargo.toml` declares `[[example]] name = "compare_aberrated_csv", test = true`, including five original **fixture-only** unit tests in `cargo test --workspace`:

- Identical CSVs including intentional duplicate bodies: zero residuals and complete report.
- Independent circular-arc longitude calculation across 360° plus detection of signed retrograde-rate discrepancies.
- Hard refusal of incorrect frame, correction label, acceleration source, center, target, epoch or tilt.
- Malformed headers, nonfinite fields, inconsistent rows, impossible light times, empty files or target-count differences.
- Bad threshold count, NaN, infinities, negatives and nonsensical strings.

All test CSV numbers are **invented fixtures**, not planetary positions. A clean CI run establishes that the comparator compiles and enforces these rules only; it cannot certify precision against real NASA/JPL/Horizons data without real reference input.

### Validation commands

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Authorized read-only Linux/Windows/macOS CI can validate this draft through a **temporary CI-only pull request targeting `dev/ephemeris`**; close that temporary PR without merging after the final head succeeds. No production promotion, package publication, external kernel downloads, GitHub secrets or additional dependencies are necessary.

## Provenance, licensing, safe integration

**CPAL-1.0 modification record 2026-11-12:** new `examples/compare_aberrated_csv.rs` with original intact Exhibit A header, `Cargo.toml` example test registration, and this English document. No astronomical arithmetic or external protected data was copied. Existing source license/NOTICE remain unchanged, and the required attribution in applicable uses is **Powered by VirgoStars** — https://www.virgostars.it/.

This draft should be stacked on **#126**, itself depending on unmerged **#118 → #114 → #107** and the separate civil-time lineage **#96/#110**. Upstream scientific validation and an explicit owner integration decision are required before merging this stack into `dev/ephemeris` or `main`. Do not deploy or present these tools as a shipping Swiss-compatible replacement.
