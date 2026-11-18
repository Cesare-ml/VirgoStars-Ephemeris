# Independent DE441 and apparent-pipeline audit — 2026-11-18

**Status: EXPERIMENTAL / sampled local validation, not production activation.**
This audit compares independent implementations and explicitly identified
Horizons outputs. It does not establish the physical orbit errors of DE441,
approve ephemeris-data redistribution, validate the app's natal chart, or
certify every epoch, target, frame, observer, rate or correction.

## Inputs, source and provenance

- Windows 11 build 26300; Rust/cargo 1.99.1, Node 22.18.0, Python 3.12,
  pyerfa 2.0.1.5. The apparent pipeline and full-file reader were tested at
  Ephemeris commit `3cd99e21b542e2400f3f6763a0e8a2c468cf7e24` plus the
  non-numerical matrix-export example in this branch. The independently tested
  pack reader was `064b9c9584732e6bad39fce55c471759c8fab473`, PR #610 based
  on #608/#601; these commits are not interchangeable or a claim of merge.
- Caller-supplied classic DE441: 2,788,676,624 bytes, SHA-256
  `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
  This matches the previously recorded fingerprint, but that fingerprint is
  not an independent authentication of the supplier. The source and slices
  were read only, not copied into Git, downloaded again or distributed.
- JPL Horizons API responses were captured on 2026-11-18 UTC. All used
  responses identified the requested target and Earth center as `DE441`.
  Observer tables said `Atmos refraction: NO (AIRLESS)` and their EOP header
  reported data through 2026-11-17 and predictions through 2027-02-12.
  `TLIST` values and returned TT dates were checked; in the calendar-formatted
  modern table the maximum rounding was 0.000489 s. Full URLs, raw responses,
  CSV references and local results were retained outside the repository;
  their checksums follow below. No birth data were sent.
- JPL's [Horizons API](https://ssd-api.jpl.nasa.gov/doc/horizons.html) and
  [manual](https://ssd.jpl.nasa.gov/horizons/manual.html) define the queried
  quantities. [ERFA](https://github.com/liberfa/erfa) is an independently
  maintained SOFA-derived implementation; comparing one model's implementations
  is not independently verifying the model's physical accuracy.

## Predeclared diagnostic thresholds and observed results

Thresholds below were chosen before each comparison and are **investigation
thresholds**, not an approved product accuracy requirement. All Horizons
comparisons use the same DE441 solution in both implementations. `Mars` through
`Pluto` refer to **system barycenters**, never body centers; Mercury and Venus
are centers, as are the Sun and Moon. Observer/center is Earth center.

| Experiment | Samples and frame | Predeclared limit | Observed maximum | Result |
|---|---|---|---|---|
| Full-file geometric states | 10 targets × 6 epochs in astronomical years 0, 1000, 2000, 3000, 4000, 5000; TDB; geocentric ICRF; `VEC_CORR=NONE` | 0.001 km and 0.001 km/TDB-day per component | 9.54e-7 km; 1.40e-9 km/day | 60/60 pass |
| Real mobile slices vs independent Horizons | 10 targets × 10 slices, one midpoint per logical interval 0–2300; TDB; geocentric ICRF; `VEC_CORR=NONE`; PR #610 Rust `validate_horizons` | 0.001 km and 0.001 km/TDB-day per component | 1.431e-6 km (Uranus barycenter, history 0800–0999); 3.726e-9 km/day (Mercury, history 0600–0799) | 100/100 pass |
| Experimental refined apparent ICRF vector | 10 targets × 61 July dates, astronomical years 0–3000 in 50-year steps; TDB core and TT Horizons Q45 | 0.05″ Moon; 0.01″ others, angular separation | Moon 0.00006229″; others 0.000005895″ | 610/610 within limits |
| Standard apparent pipeline, frame-independent angular separations | 10 bodies, 25 reception dates 1900–2094, 45 body pairs per date; Horizons Q2 and Rust true equator/ecliptic of date | 0.05″ pairs with Moon; 0.01″ without | 0.0007503″ with Moon; 0.000008142″ without | 1,125 pair/date samples within limits |
| Standard apparent ecliptic of date | 10 bodies × same 25 dates; Horizons Q31 IAU76/80 of-date versus Rust IAU 2006/2000B of-date | 0.5″ per longitude/latitude component | 0.248552″ longitude, Pluto barycenter at JD TDB 2415100.5; 0.044629″ latitude, Jupiter barycenter at JD TDB 2418061.2 | 250/250 within these broad limits |
| Frame rotation vs ERFA | 25 TT dates in 1900–2094; ICRF → true ecliptic/equinox of date | 1e-12 per matrix element | 2.221e-16 | 25/25 pass |

The geometric comparison has no light-time, aberration, deflection,
precession, nutation, topocentric or atmospheric corrections. Each slice was
also checked against the v1 manifest's recorded byte count and SHA-256 before
and after use. One midpoint is **not** exhaustive coverage of any slice; the
separate #610 regression checks logical starts and immediately preceding ends
against the same full source, which is not independent reference validation.

The apparent Q45 experiment uses the **experimental refined fixed-ICRF** path,
including light-time, solar gravitational deflection and annual aberration;
it does not validate any ecliptic frame. The Q2/Q31 experiments use the
**standard** six-field pipeline, not that experimental path. Both use a
geocentric observer and airless reference, with outer-planet barycenters.
The standard path computes TT from TDB with a documented short periodic
approximation; Q31 queried Horizons at the TT emitted by Rust to nine decimal
places. The mean calendar-date rounding of the Q2 table is not an independent
time-scale validation.

Horizons describes Q31 using IAU76/80 ecliptic of date, while the core uses
IAU 2006 bias/precession plus IAU 2000B nutation and true obliquity. The broad
0.5″ Q31 comparison therefore does **not** mean the two frames are identical,
nor does it establish sub-milliarcsecond accuracy. The Q2 absolute differences
were at most 0.2438″ in RA·cos(Dec) and 0.0930″ in declination, each below the
predeclared 0.5″ diagnostic limit; differing precession/nutation and EOP
conventions prevent attributing the entire residual to either implementation.
Pair separations remove a common rigid rotation, not other systematic effects.

For ERFA the reference was
`rx(obl06 + deps, numat(obl06, nut00b) @ pmat06)` with the same TT date,
using `pyerfa` and no ephemeris state or apparent corrections. The Rust
example `audit_true_ecliptic_matrix` exports the actual 3×3 matrix. This tests
implementation agreement for a shared model only; IAU 2000B and Horizons'
model have their own limitations and errors.

## Existing baseline regression — **FAIL**, not silently updated

`cargo test --release --offline --test apparent_ecliptic_of_date -- --ignored`
on the tested main commit yielded **3 passed, 1 failed**. The failure is
`real_de441_matches_committed_engine_baseline`, an internal historical
same-engine regression, not an independent reference. The same test and kernel
pass at the immediate pre-change commit `6835e00e4b4ba5fc52d882a793e2dfa2b2a436aa`
and fail at `ff26f5ab0bcfaed0f0b8b76fbf5c2392de5be400`, which tightens
light-time convergence from four epsilon-scaled units to one. Across the
1,990 baseline rows, **137 Moon cases** exceed at least one old tolerance;
no other target does. Largest longitude drift is 4.31143e-7 degree
(about 0.001552″), with maxima in the other five fields recorded locally.
The old baseline was retained, not overwritten or represented as current.

To investigate rather than infer correctness, at two Moon samples the old
and current **standard** apparent equatorial outputs were rotated back into
ICRF with ERFA and compared with Horizons Q45. At JD TDB 2415100.5, angular
residuals were 0.0009820″ old versus 0.00009094″ current; at 2481949.1,
0.0013104″ old versus 0.0002418″ current. Both were below the predeclared
0.05″ diagnostic limit. These two cases support investigating the convergence
change; they do not establish that all 137 changed cases improved, justify
rewriting historical evidence, or remove the failing test/review gate.

## Executed checks and evidence

- `cargo fmt --all -- --check`: pass; `cargo test --workspace --offline`:
  **508 passed, 0 failed, 31 ignored** in the ordinary run. The focused real
  apparent run above includes the one explicitly reported failure; the
  four-body Horizons light-time tests passed 2/2 and the external-file DE441
  reader test passed 1/1. Parser/grid Node tests passed 4/4.
- `cargo clippy --workspace --all-targets --offline -- -D warnings`: pass.
  `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps --offline`:
  **fail** on pre-existing bare URLs in `fixed_star.rs` and `gmst_iau2006.rs`;
  these are not failures of the numerical comparisons.
- The full-file geometric tool is
  `node bindings/node/scripts/compare-horizons-geometric-range.cjs <kernel> <sha256> <new-report>`.
  The refined ICRF tool is `node bindings/node/scripts/compare-horizons-apparent-icrf.cjs <kernel> <sha256> <new-report> 0-3000-step-50`.
  The standard modern tool is `node bindings/node/scripts/compare-horizons-apparent.cjs <kernel> <new-report>`;
  its returned aggregates do not by themselves preserve raw responses or check
  its results against thresholds, so this session captured/checked the exact
  requests, metadata and rows separately. Its source has a hard-coded
  historical EOP interval: the audit does not treat that interval as the
  current Horizons coverage.
- The additional local orchestration files `compare_packs_horizons.py`,
  `compare_ecliptic_horizons.py`, `compare_orientation_erfa.py`,
  `check_baseline_horizons.py`, `compare_baseline.py` and the predeclared plan
  were saved alongside the raw-response evidence on the owner's machine; they
  invoke the existing Rust core or ERFA and do not implement a second engine.
  They have local absolute paths and must be adapted before reuse elsewhere.
- Local evidence SHA-256: `horizons-geometric.json`
  `29f1eef7bc5be8b445cffecfef264377701e4570a4943d10b8d4ea771ae5340e`;
  `horizons-packs.json`
  `a2c13480460aaf5debab6d72c995de010bab009a7b53042c3b815ea52e966a33`;
  `horizons-apparent-icrf.json`
  `0d9261a0f9f76c87b5c78c85f5b2b493a4eac6d45166a61fa408b8ec97d4bb8e`;
  `horizons-apparent-ecliptic.json`
  `b20b570188abcdc792407ea0876b060106a76c1827cc26fb6e47ee9fae19373f`;
  `horizons-modern-responses.json`
  `93fec2ad7410d1b8718ed5291000782be667958e8cdc7237c7183cb46158cd88`;
  `orientation-erfa.json`
  `59f8c8ef7415733831048e25a334233094aa1b33b1465037e268a5a974af7217`;
  `baseline-drift.json`
  `4d8f71a8d01b45ba5a7d8cc0c0a2bef77b1fe3cb6501f1c5001ef10d22e9ccaf`;
  `baseline-horizons-investigation.json`
  `daa70dd4525d50cf69f51300fdbea063cdb2f5b043e4faaeab9c9ce9455a0149`.

## Remaining gates

Resolve the stale failing baseline with a reviewed, versioned regression
strategy; do not silently loosen the threshold. Obtain an independently
authenticated kernel provenance, review JPL data distribution rights, extend
sampling to boundaries and additional epochs/targets, and verify independently
defined angular/radial rates, time conversions and observer conventions.
Historical civil time, topocentric observations, planetary deflection,
planet-center offsets, physical ephemeris uncertainty and Android runtime
remain outside this audit. Do not enable this in Mobile from these results.
