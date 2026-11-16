# MVP 1, row 1 — geocentric apparent ecliptic of date, with rates

**Status: Locally tested on a real classic DE441 file; compared locally with an
independent reference (see below). NOT integrated, NOT released, NOT a complete
MVP 1.** Branch: `integration/mvp1-apparent-pipeline-20261116` (based on
`dev/ephemeris`, no change to `main`).

This closes the main technical gap named in the
[three-MVP document](THREE_MVP_SWISS_REPLACEMENT_2026-11-11.md): the aberrated
six-field stack only produced *fixed-axis* (ICRF) coordinates. The new module
`apparent_ecliptic_of_date` rotates that state into the **true ecliptic and
equinox of date** and converts it to the six consumer fields.

## Scientific contract

| Item | Value |
|---|---|
| Input epoch | TDB Julian date (`JulianDateTdb`) |
| Center / observer | Earth center (geocentric) |
| Targets | Sun, Moon, Mercury, Venus, and the *system barycenters* of Mars–Pluto; a barycenter is never relabeled a planet center |
| Frame of output | true ecliptic and equinox of date: IAU 2006 bias-precession (Fukushima–Williams), IAU 2000B nutation (77 terms), true obliquity |
| Corrections applied (default) | one-way light-time, **first-order solar light deflection** (before aberration), special-relativistic annual aberration (analytic Earth acceleration), frame rotation |
| Corrections NOT applied | planetary light deflection (Jupiter < 0.0002″), topocentric parallax / diurnal aberration, barycenter→body-center offset |
| Option | `ApparentOptions { solar_light_deflection }` (default `true`); the CLI flag `--no-solar-deflection` turns it off, and every result carries `solar_deflection_applied` |
| Units | degrees, AU, and degrees (or AU) per **TDB** day |
| Rates | `v' = M v + (dM/dt) r`; `dM/dt` is a central finite difference (half-step 0.01 TT day) of the declared matrix |
| TT for the matrix | `TDB − 0.001657 s · sin(g)` (leading periodic term; omitted terms < 30 µs) |
| Validated range | 1900–2100 TT. Outside it the call returns `ApparentError::Orientation` (fail closed, no extrapolation) |

Entry points: `geocentric_apparent_six_fields`,
`geocentric_apparent_six_fields_many` (Earth read once, all-or-error), their
`*_with_options` forms, and the CLI
`cargo run --release --example apparent_six_fields -- [--no-solar-deflection] <de441.eph> <TDB-JD> [Body ...]`.
Solar deflection lives in `light_deflection` (single-body weak-field formula,
Explanatory Supplement §7.2.4 / Klioner 2003, implemented from the formula).

## Defect found and fixed on the real kernel

`light_time::geocentric_light_time` required the fixed-point iteration to
reach *exactly* equal `f64` epochs. For the Moon (≈ 1.3 s delay) at JD ≈ 2.4e6 the
iteration alternates between two adjacent representable JDs (1 ulp ≈ 40 µs) and
returned `DidNotConverge` — for example at TDB JD 2436036.6. The synthetic tests
never exercised this. Convergence is now accepted at ≤ 4 ulps of the observation
epoch; the returned state is the one evaluated at the final emission epoch. The
regression test is `real_de441_moon_light_time_converges_across_1900_2100`.

## Evidence actually obtained (2026-11-16, Windows 11, Rust 1.99.1)

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo test --workspace`: pass on the integrated branch.
- With a locally held classic `de441.eph` (not in this repository, not
  downloaded by the code): `DE441_PATH=<file> cargo test --release --test apparent_ecliptic_of_date -- --ignored`
  → 3 passed. They check 10 bodies on 60 dates in 1900–2100 TDB against
  exact-step (2⁻⁸ day) centered differences of the module's own positions
  (this verifies rate consistency, not absolute accuracy) and J2000 Sun/Moon
  longitudes against public almanac values at the 1e-3° level.
- **Independent reference, local only.** A throw-away probe (never committed)
  evaluated the Swiss Ephemeris C library, AGPL, in its JPL mode on the *same*
  `de441.eph`, with the TDB epoch passed as its ephemeris time, at 199 dates
  from JD 2415100.5 to 2488000 (step 367.3 d), for the same ten targets.
  Both configurations were run with matching flags: Swiss default (deflection
  on) against this module's default, and Swiss `SEFLG_NOGDEFL` against
  `--no-solar-deflection`. The maximum difference over all ten targets and
  199 dates was identical in both configurations:

  | Quantity | Max abs difference |
  |---|---|
  | longitude | 0.00067 arcsec |
  | latitude | 0.00055 arcsec |
  | distance | 2.4 m worst case without deflection (Mercury); 37 m with it (Uranus, at 20 AU) |
  | rates vs centered difference (±0.001 day) of Swiss *positions* | longitude ≤ 0.002 arcsec/day, latitude ≤ 0.0003 arcsec/day, distance ≤ 1.3 km/day for all bodies except the Moon (longitude 0.28, latitude 0.02 arcsec/day, distance 0.5 km/day) |

  Before the deflection stage existed, the difference from Swiss default grew
  to 0.07–0.5″ in longitude and 0.03–0.9″ in latitude for the planets (largest:
  Uranus). Adding the stage brought it back to the 0.0007″ floor, so the stage
  and its rates agree with the reference. The remaining 0.0007″ is not
  attributed to a specific cause.

  Swiss's own reported speeds disagree with the centered difference of its own
  positions by up to 0.08 arcsec/day (latitude) and ~1e4 km/day (distance), so
  they are not usable as the rate reference; the positions are.
- The Moon's rate difference (0.28 arcsec/day ≈ 1e-5 relative) was not
  separated from the finite-difference noise of the comparison; it is recorded,
  not explained.
- Solar deflection: closed-form unit tests (cot(ψ/2) law, direction, scaling
  with distance, source in front/behind the Sun, invalid distance) in
  `tests/light_deflection.rs`; with the real kernel the deflection of Sun and
  Moon is below 0.001″ and of every body below 2″ on 199 dates, and the rate
  test above runs with deflection on.

These residuals describe agreement between two implementations on one kernel.
They are not a JPL Horizons validation and do not certify absolute accuracy.
The Swiss comparison is for the maintainer's local use only (no professional
Swiss licence is held); no Swiss code, binary or output file is part of this
change.

## Still open for MVP 1 (not done here)

1. Planetary light deflection (negligible here, < 0.0002″) and the rate of the
   solar deflection being a finite difference rather than analytic.
2. Independent JPL Horizons reference comparison of the apparent quantities.
3. Civil input is done for 1900-01-01 … 2027-06-27 UT (see "Civil front door"
   below); outside it, and before the first leap-second era with measured
   UT1−UTC, nothing is extrapolated. Not covered: UT1−UTC (taken as 0 after
   1972), dates before 1900 and after 2027-06-27.
4. Row 2 remainder: the Swiss-shaped `swe_julday`/`swe_revjul` return fields in a
   facade (the arithmetic exists in `calendar_decimal_hours`), and ΔT outside
   1900–1972.
5. Row 3: houses (`astrological-functions`) — separate repository.
6. Row 4: fixed stars (catalog rights review first).
7. Row 5: Node/WASM binding and load interface. Branches exist
   (`codex/node-*`) but were not reviewed or merged here.
8. Epochs outside 1900–2100, and non-DE441 data (`.bsp`), are unsupported.
9. Integration gate: parallel run, documented rollback, owner approval. Nothing
   here is a production switch.

## Provenance

The 77-term IAU 2000B coefficients and ERFA conformance values come from ERFA
(BSD-3) and are listed in `THIRD_PARTY_NOTICES.md`. The orientation module was
taken unchanged from `feat/apparent-geocentric-20261110`; the apparent module
composes existing stages and adds only the single-term TT−TDB expression above
and the solar `2GM/c²` constant (1.97412574336e-8 AU, the IAU nominal solar GM).
No Swiss, SOFA or ERFA code was copied; the deflection formula was written from
the published equation.

## Civil front door and historical ΔT (added 2026-11-16)

`civil_apparent::geocentric_apparent_six_fields_many_from_civil` takes a civil
timestamp with an explicit UTC offset, converts it once under a **declared and
reported** policy and returns the six fields plus `time_source`:

| Instant (UT) | `time_source` | Method |
|---|---|---|
| 1972-01-01 … 2027-06-27 | `IersLeapSeconds` | existing TAI−UTC table, TT, TDB series; UT1 taken as UTC (|UT1−UTC| < 0.9 s) |
| 1900-01-01 … 1972-01-01 | `DeltaTPolynomial { model_id, delta_t_seconds }` | `TT = UT + ΔT`, ΔT from the Espenak–Meeus 2006 polynomials (`delta_t` module), TDB = TT + leading periodic term |
| anything else | error | no extrapolation, no fallback |

The UTC offset decides the policy, not the local calendar date. Leap-second
labels and invalid dates are rejected. Checks: the ΔT fits are continuous at
their joins (< 0.1 s), join the exact 1972 value TT−UTC = 42.184 s within
0.1 s, and the two policies agree to < 0.1 s across 1972-01-01. The ΔT fits are
smoothed history: real ΔT differs by a few tenths of a second before about
1955, which is about 0.2″ for the Moon per 0.4 s. That uncertainty is not
removed, only reported through `delta_t_seconds`.

## Evidence stored in the repository

- `docs/evidence/mvp1-apparent-local-reference-receipt-2026-11-16.json`: the
  aggregate residuals of the local comparison above (grid, flags, versions,
  per-body maxima, what is *not* established). It holds **no** reference-library
  output rows, code or binaries, so nobody needs to rerun that comparison to
  read the result, and the project does not redistribute AGPL output.
- `docs/evidence/mvp1-apparent-baseline-1900-2100.csv`: 1990 rows (199 dates ×
  10 bodies) produced by **this engine**. The ignored test
  `real_de441_matches_committed_engine_baseline` recomputes them from a local
  DE441 and fails on any change above 1e-9° (or 1e-12 AU), so regressions are
  caught without the reference library.

The test suite itself stays free of external files: everything that needs the
2.8 GB kernel is `#[ignore]` and selected by `DE441_PATH`.
