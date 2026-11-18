# Station and orientation-boundary rate audit — 2026-11-18

**Result: the corrected JPL/ERFA position- and range-derivative comparisons meet the predeclared limits at all sampled station and boundary points. Direct Q20 radial rates fail their separate threshold in some cases.** The first analysis also exposed a bug in the comparison script's treatment of rounded epochs; both the original failure and the replay are retained below. No Mobile engine is enabled. The [sampling plan and limits](STATIONS_BOUNDARY_RATE_PLAN_2026-11-18.md) were fixed before querying Horizons.

## Scope and method

The standard Rust apparent six-field path uses the locally held classic DE441, geocentric apparent positions and true ecliptic of date. JPL Horizons supplies DE441 geocentric airless apparent ICRF Q45 positions and Q20 distances and radial rates. ERFA rotates each Q45 direction at its own TT epoch using the same IAU 2006/2000B orientation model, so the reference derivative includes motion of the frame. Rust and Horizons share DE441; ERFA and Rust share the chosen orientation model. This compares implementations and selected numerical behavior, not the independent physical accuracy of the orbit or Earth orientation.

For Mercury, Venus and Mars–Pluto system barycenters, a five-day scan in 2024–2026 found the first Rust speed change for each direction and each body: **16 station candidates**. Rust bisection gave brackets no wider than 0.0001 TDB day; these are Rust-selected locations, not JPL-certified roots. Each candidate was checked at its midpoint and ±1 and ±6 hours, giving **80 station samples**. Fourth-order position/range derivatives use h=0.02 and 0.04 TDB day.

At the temporal bounds, the orientation primitive permits TT JD **[2415020.0, 2488070.0)**: 1899-12-31 12:00 TT inclusive to 2100-01-01 12:00 TT exclusive. It does **not** cover the rest of calendar year 2100. The six-field rate additionally needs the matrix at TT ±0.01 day. For the ten targets, test four offsets inside each bound, nearest nominally 0.0101 TDB day, for **80 boundary samples**. Their reference stencils use h=0.0025 and 0.005 day, and all actual TT samples were checked to lie within the half-open orientation interval. This does not establish continuity or correctness for every instant, and does not extend the model's guard.

Horizons input TT is the existing Rust approximate TDB→TT conversion rounded to nine decimal places. The corrected derivative uses the **actual requested TT samples** as abscissae; reference results are per TT day, compared with Rust's per TDB day. The core's leading-term model has a relative TT/TDB rate difference below 1e−9, which is a residual systematic, not an exact equality or a full relativistic time-scale uncertainty. Additional limits are printed-angle and distance precision, finite stencils, single-f64 Julian dates, Q45 model differences and possible light-deflection model errors.

## Numerical results

Both stencils are counted for each sample. Component limits of 0.05″/day for longitude/latitude and 5e−10 AU/day for radial rate were fixed before the reference fetch and are diagnostic, not release budgets.

| Cases | Longitude maximum / failures | Latitude maximum / failures | Radial maximum / failures |
| --- | ---: | ---: | ---: |
| Station windows, 160 checks | 0.0001325″/day; 0/160 | 0.0001093″/day; 0/160 | 9.624e−12 AU/day; 0/160 |
| Boundary windows, 160 checks | 0.001292″/day; 0/160 | 0.0009637″/day; 0/160 | 2.529e−11 AU/day; 0/160 |

For each of the 16 station candidates, Rust and the JPL/ERFA finite-difference estimate had the expected opposite signs at both **±1 hour and ±6 hours**, for each stencil: 16 confirmed, 0 indeterminate and 0 contradicted for every window/stencil combination. Sign was evaluated only when both absolute rates exceeded the predeclared 0.0005″/day deadband. Across the 160 station-point/stencil checks there were zero opposite nonzero signs and **20 indeterminate signs**, including points close to the Rust root. This is a sampled sign comparison around Rust-selected roots, not a certified unique JPL station time or uncertainty interval.

The largest difference between the two reference stencils was 0.0001504″/day longitude, 0.0001384″/day latitude and 5.648e−12 AU/day radial near stations; at boundaries it was 0.001258″/day, 0.001204″/day and 3.560e−11 AU/day. These differences are sensitivity indicators, not error bounds. The closest non-Sun station sample has solar elongation 14.6659°; the closest non-Sun boundary sample is Moon at 2.034°. These data do not validate behavior at the solar limb or an occultation.

For six-field rate calls at each exact primitive bound, 0.005 day inside, and 0.0001 day outside, all **60/60** target/bound/probe calls returned a typed `Orientation(UnsupportedEpoch)` error; no fallback was observed. The matrix primitive can still be valid within 0.01 day of a bound: its derivative requires the additional stencil. All sampled centers 0.0101 day inside both sides succeeded. Exact transition details depend on the TT conversion and binary floating-point rounding. A separate synthetic Rust test exercises the matrix-versus-rate guard.

## Direct Q20 radial-rate discrepancy remains open

Against the derivative of Q20 range, the predeclared separate 0.001 km/s diagnostic limit is exceeded in **20/160 station checks** (10 sample points, Mercury) and **22/160 boundary checks** (11 sample points, Mercury/Venus). Largest absolute differences are **0.005201 km/s** and **0.004150 km/s**, respectively. The script therefore saves its report and exits **1**; this is a genuine failed direct-rate comparison, not a failed fetch. The earlier [Q20 investigation](DE441_ECLIPTIC_RADIAL_RATE_VALIDATION_2026-11-18.md) found numerical evidence consistent with a different emission-time-derivative convention; JPL has not confirmed the internal formula.

## Correction to the first analysis

The initial script incorrectly formed finite-difference weights from unrounded TDB timestamps while Horizons was queried at rounded TT timestamps. The short boundary stencils amplify that time-grid mismatch. In the initial saved report, the unchanged 5e−10 AU/day radial limit failed **1/160 station** and **63/160 boundary** checks. For Mercury near the lower bound at JD TDB 2415020.03 with h=0.0025, the initial residual was −5.6874e−9 AU/day. Replaying the *same* retained JPL responses with weights from their actual requested TT epochs gives **−7.3865e−12 AU/day** for that point and 0/160 failures in both groups. The 1,120 returned reference timestamps matched the requested TT values as represented in the replay. The ±TT-rate versus TDB-rate systematic remains as stated above.

The station script and the earlier general-grid script now use actual requested TT abscissae. An offline replay of the earlier **90-case** grid produced the same published maxima and the same 22/180 direct-Q20 failures; it does not turn those failures into passes. A corrected fresh Horizons execution reproduced the station/boundary summaries, but reused the same selected events and ephemeris; it is not a new holdout. The exact original report is preserved for review, and a numerical unit test now distinguishes actual and incorrectly assumed timestamps.

## Reproduction and local evidence

Supply the caller-owned classic DE441 (SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`), Python 3.12 with pyerfa 2.0.1.5 and NumPy 2.5.5, and network access for the first command. From the repository root:

```text
cargo build --release --offline -p virgostars-ephemeris-core --example audit_station_rate_points
python scripts/audit_station_boundary_rates.py <kernel> <kernel-sha256> <example-exe> <new-report.json>
python scripts/audit_station_tt_replay.py <kernel> <kernel-sha256> <example-exe> <old-report.json> <old-report-sha256> <new-replay.json>
python scripts/audit_rate_grid_tt_replay.py <old-grid.json> <old-grid-sha256> <new-grid-replay.json>
python -m unittest discover -s scripts -p test_station_boundary_numerics.py -v
```

Reports are written to new caller-selected paths, with input hashes checked. The two audit commands exit **1** when the separate direct Q20 threshold is exceeded, after retaining the complete report. Raw Horizons requests/responses and the kernel remain local and are not committed. SHA-256 of local evidence:

- Original first report: `f807fda825cbb3b16bbd55e352e19c218b3bda4ea554fab9d5d3b1f9175dc4fd`.
- Offline TT replay of that report: `b5bf2665248e821f2098bb2871eea6bae8443eefb26c3dde8104608fe43bd74c`.
- Corrected fresh-reference report: `d71df040f01e028af6e4635f5d57d6bdb1b3edd62ee8d2e28cda0bf1b63b15f4`.
- Earlier 90-case grid TT replay: `07bcbaccbd154228be198c4406682af1dc7726b0503f379e239140268d27c5a9`.

Windows checks: `cargo test --workspace --offline` passed 509 tests with 31 ignored; four ignored real-DE441 apparent tests passed separately. Rust fmt and Clippy passed, and three Python numerical/deadband tests passed. The JPL convention, continuous accuracy, exact station roots, planetary centers beyond the inner planets and physical ephemeris uncertainty remain open. The prepared JPL inquiry is still unsent.
