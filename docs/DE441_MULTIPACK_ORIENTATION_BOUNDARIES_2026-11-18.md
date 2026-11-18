# DE441 long-term orientation and multipack handoffs — 2026-11-18

**Status: exploratory; provider-backed apparent pipeline not implemented.** At all nine internal package boundaries the real manager selected the expected package and the existing light-time solver, driven through manager state requests, agreed with the full file for the sampled cases. The experimental long-term pipeline itself still accepts one `De441` reader and does not use the manager for each state request. A reception-only adapter would violate the intended logical pack policy in tested missing-emission cases. Do not activate it in an app or claim the complete multipack apparent pipeline has passed.

The [test plan and thresholds](DE441_PROVIDER_BOUNDARY_PLAN_2026-11-18.md) were recorded before the corresponding runs. The long-term model and earlier JPL/ERFA comparison are described in [the preceding assessment](DE441_ORIENTATION_EXTENSION_ASSESSMENT_2026-11-18.md).

## Setup and actual integration boundary

The orientation/core code is commit `1a2938c490f0159b4506ab00e44bdda381e25ce6` (#626). The actual manager source is extracted unchanged from commit `064b9c9584732e6bad39fce55c471759c8fab473` (#608 audit, containing #601's manager); its Git blob content has SHA-256 `1191e46208c244bcfdc9d0940b2f429e662d78d4092bba3e65131b450a0bad6b`. A separate diagnostic crate compiles that exact source against the #626 core. This cross-branch harness is not evidence that either PR already contains the other's implementation. `light_time_with_states` calls the manager for each requested epoch; because the solver currently accepts `De441Error` rather than `De441PackError`, the **test harness** separately retains and inspects the original typed manager error and maps it to an I/O error solely for the solver callback. The existing public apparent API does not provide this adapter.

Manifest v1 specifies ten half-open JD **TDB** ranges, year 0 meaning 1 BCE; the internal boundaries are 1 January of years 200, 400, 600, 800, 1000, 1200, 1400, 1600 and 2101 in proleptic Gregorian TDB. The ordinary orientation guard still ends at 2100-01-01 12:00 TT, exclusive. The longer model uses Vondrák poles and extrapolated 77-term nutation with full IERS arguments; it is not standard 2000B. Light-time, solar deflection, annual aberration, first-order emission refinement and moving-frame rate are experimental; time inputs are TDB and approximate TT, not civil time.

## Local provider and transition results

At each boundary, offsets −0.08, −0.02, −1 second, 0, +1 second, +0.02 and +0.08 TDB day were evaluated for Sun, Moon, Mercury, Venus and the Mars through Pluto **system barycenters**, geocentric for the apparent result. This is **630 target/epoch cases**. All selected IDs matched the half-open catalog, including the exact boundary and cache changes. The provider's barycentric state plus acceleration and the manager-driven light-time position, velocity and emission epoch agreed with the full-source reader in 630/630 cases: maximum floating-point differences were zero for each checked component, below the planned 1e−6 km, km/day or km/day² and 1e−9 day limits. This is coefficient identity against the same file, not independent physical validation.

For the same 630 cases, running the long-term six-field function on a reader opened from the **reception-selected file** agreed with running it on the full file (all six floating-point component differences zero; limits 1e−8 degree or degree/day and 1e−12 AU or AU/day). That is a separate single-reader comparison, not the manager-driven apparent pipeline. In **252/630** manager-driven light-time solves, at least one request used another logical package; crossing includes the solver's intermediate requests, not just final emission.

For 90 boundary/body pairs, the apparent position change from −1 to +1 second was corrected for the trapezoidal average of the reported endpoint rates over the actual representable TDB interval. Maximum absolute continuity residuals were **4.991e−12 degree longitude**, **3.705e−12 degree latitude**, and **3.441e−14 AU distance**, below the planned 1e−8 degree and 1e−12 AU diagnostics. This detects sampled discontinuities after accounting for ordinary motion; it is not an independently measured Earth-orientation error or a continuous bound. The model's internal frame derivative evaluates orientation at TT ±0.01 day; that rotation does not itself request a DE441 state at those epochs. The separately tested ±0.02/0.08 TDB days represent observation-side probes for numerical-reference stencils.

## Missing, corrupt and uncovered data

Using caller-defined catalog paths pointing to a non-existent test location, without changing real packages:

- **9/9 missing reception packs** returned `De441PackError::MissingPack` with the expected ID.
- **9/9 missing prior emission packs**: Earth at reception was available, but the manager-driven Pluto light-time solve failed with the prior package's typed `MissingPack` retained by the harness. There was no manager fallback.
- In the same 9/9 scenarios the current **single-reader** experimental apparent function succeeded using only the reception-selected file. This is possible because exported records overlap the logical interval. It demonstrates why merely choosing a reception file and passing it to the existing apparent function does **not** enforce the multipack contract; no production adapter was added.
- One tiny, separately created corrupt test file yielded `Reader`; epochs just before the first logical interval and at the last exclusive end yielded `NoPackForEpoch`; NaN yielded `InvalidEpoch`. The manager's inventory checks existence, not a file's SHA-256. Installation, authenticated catalog, live replacement and Android were not tested.

This logical-coverage policy requires a separately reviewed provider-aware pipeline, including all light-time iterations, target acceleration at emission, reception Earth/Sun states, the refinement's sub-ULP coverage treatment, typed error propagation and tests for package changes. If a policy instead deliberately permits physically overlapping records when the logical neighboring pack is absent, that is a different product contract which must be made explicit and reviewed; the existing manager does not do so.

## New JPL/ERFA comparisons at exact boundaries

At the nine exact boundaries, 10 targets were compared with [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html) observer Q45, Earth center `500@399`, ICRF, airless apparent directions and Q20 distance. [ERFA](https://github.com/liberfa/erfa/blob/master/src/ltp.c) rotates Q45 using independently coded Vondrák poles, J2000 bias and full IAU 2000A nutation. Five TT epochs per target/boundary (center, ±0.02, ±0.04 day) give fourth-order derivatives with the actual rounded TT abscissae. Candidate input is TDB, converted approximately to TT. The candidate here uses the full source reader, not the manager-backed apparent API, which does not yet exist.

| Quantity | Max absolute residual | Predeclared diagnostic limit | Exceedances |
| --- | ---: | ---: | ---: |
| Longitude | 0.0008354″ | 1″ | 0/90 |
| Latitude | 0.000004701″ | 1″ | 0/90 |
| Distance | 1.659e−11 AU | 1e−9 AU | 0/90 |
| Longitude rate | 0.001917″/day | 0.1″/day | 0/90 |
| Latitude rate | 0.0002283″/day | 0.1″/day | 0/90 |
| Distance rate | 4.113e−10 AU/day | 5e−10 AU/day | 0/90 |

The radial maximum is relatively close to the exploratory bound; the bound is not a calibrated scientific uncertainty. Minimum non-Sun elongation was 0.4577°: a near-Sun numerical stress case, not a validated solar-limb visibility or deflection accuracy. Direct Horizons Q20 `deldot` differs from the derivative of its own distance beyond 0.001 km/s in **10/90 cases**, maximum **0.008846 km/s**; JPL's convention remains unconfirmed. The script records the complete result and exits 1 for these failures. The two sides share DE441, and ERFA's nutation model differs from the candidate's: these cross-model comparisons neither validate physical orbital truth nor establish accuracy throughout 0–2300.

## Reproduction and evidence

On Windows 11, Rust `cargo 1.99.1` / `rustc 1.99.1`, the caller's source and all ten local slices were checked for size and SHA-256 before and after the provider run; manifest SHA-256 `a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a`, source SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`. No data files were copied or distributed; rights remain `review_pending`. A tiny synthetic corrupt file is created only in the new diagnostic build directory.

```text
python scripts/run_multipack_boundary_audit.py <manager-repo-containing-commit-064b9c9> <kernel> <manifest> <pack-dir> <new-work-dir>
cargo build --release --offline -p virgostars-ephemeris-core --example audit_pack_orientation_candidate
python scripts/audit_boundary_horizons_erfa.py <kernel> <manifest> <candidate-exe> <new-horizons-report.json>
cargo fmt --all -- --check
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```

Horizons comparison requires network, Python, NumPy and pyerfa 2.0.1.5; URLs and raw replies are retained locally. The provider result `results.csv` SHA-256 is `ff0943e992abfbb12225490b7f6a0860fe50aad1d9a9af0526ee4ae6d4103980`; `multipack-boundary-horizons.json` SHA-256 is `0f7b41c20a1bcda268475a907c1fbc34aa49735aa113d622aefeb6addb8fbb6f`. Workspace tests: **510 passed, 0 failed, 31 ignored**; fmt and Clippy passed, plus diagnostic Rust formatting and two Python syntax checks. No ignored test was executed in this particular run. The temporary harness built and all of its assertions passed; it is not part of the workspace suite. No merge, release, Android manual test or app activation occurred.
