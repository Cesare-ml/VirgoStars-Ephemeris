# Horizons Q46 versus the derivative of Q45 — 2026-11-18

**Result: the discrepancy is reproducible and is quantitatively explained, on the sampled cases, by different treatment of correction derivatives. Horizons' internal implementation is not publicly established by these tests.** No production code, app integration or runtime default is changed. This is a follow-up to [the lunar baseline and rate audit](DE441_LUNAR_BASELINE_AND_RATES_2026-11-18.md); its plan was [recorded before the new queries](Q46_INVESTIGATION_PLAN_2026-11-18.md).

Follow-up: [separate aberration and deflection comparisons with ERFA](Q46_ABERRATION_DEFLECTION_ERFA_2026-11-18.md) include a prepared, unsent JPL inquiry and additional explicitly limited numerical evidence.

## Contract and controls

Horizons [defines Q45 and Q46](https://ssd.jpl.nasa.gov/horizons/manual.html) as airless apparent ICRF coordinates and rates with light-time, solar deflection and stellar aberration; Q46 prints RA rate times cos(dec), and declination rate, in arcseconds/hour. Raw response headers and column labels were checked for the requested target, center, ICRF, TT and DE441. The existing angular conversion explicitly handles the RA wrap and cos(dec), and divides per-TDB-day rates by 24. Inputs to the engine are JD TDB; Horizons requests use approximately corresponding JD TT. The small one-term TT/TDB conversion, printed coordinates and rates, and single-f64 JDs limit this comparison. Both systems use DE441: the computations are independent, the underlying orbital solution is not.

On the existing 610 geocentric cases (ten targets, astronomical years 0–3000 in 50-year steps), Q46 differed from the refined experimental Rust rate by more than the predeclared 0.01″/hour component threshold in 243 cases. Q45 centered differences agreed with the same Rust rate within the separately predeclared 0.005″/hour component threshold in all 610. Three new Q45/Q46 queries of six selected cases used half-steps 0.0005, 0.002 and 0.01 TDB day. All six Q45 component spreads were below the predeclared 0.005″/hour stability limit; the three lunar cases still exceeded 0.01″/hour in at least one component. This challenges a step-size explanation but is not an uncertainty estimate.

## Decomposing the rate

For a receiving observer, with target emission time `te(t)`, the light-time-only position is `r(t) = R_target(te(t)) − R_observer(t)`. Its derivative is `v = V_target(te) * (1 − dLT/dt) − V_observer(t)`. The [NAIF aberration documentation](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/abcorr.html) distinguishes derivatives of corrected positions from merely corrected positions. Further terms arise by differentiating stellar aberration and solar deflection. The Rust example `diagnose_icrf_rate_components` already supplies controlled variants. Each variant below is compared with Q46 as a two-component vector; RMS and max are in ″/hour. These 610 cases were previously used to form the hypotheses: this is an in-sample decomposition, not a new holdout.

| Diagnostic | RMS | Maximum |
| --- | ---: | ---: |
| Full refined apparent derivative | 0.0413196 | 0.1932965 |
| Full unrefined derivative | 0.0413195 | 0.1932953 |
| Omit Earth acceleration alone | 0.0441197 | 0.2074984 |
| Omit emission-time rate alone | 0.0407979 | 0.1932713 |
| Omit both of the previous terms | 0.0436703 | 0.2074746 |
| Apparent position, velocity before aberration derivative | 0.00373555 | 0.0174541 |
| Also omit emission-time rate | 0.000713602 | 0.0107518 |
| Also omit solar-deflection rate | 0.000117000 | 0.000663470 |

The last deliberately inconsistent hybrid projects `V_target(te) − V_observer(t)` using the apparent position, without differentiating the three corrections. It is **not** the derivative of the apparent direction and must not be used as a physical rate. Its agreement with Q46 is strong numerical evidence for the above interpretation, not proof of JPL's source code or an exhaustive explanation of the remaining residual. Across the same 610 cases, the vector predicted by `full − hybrid` reproduces 99.9959% of the uncentered squared magnitude of the observed `Q45 finite difference − Q46` vector; the remaining RMS is 0.000265″/hour. This descriptive in-sample ratio is neither statistical confidence nor physical accuracy. An [earlier 2,400-case holdout and solar-separation investigation](SOLAR_RATE_OUTLIERS_AND_HISTORICAL_TIME_2026-11-17.md) also found similar behavior; that earlier study was not rerun here.

## A targeted observer and light-time experiment

The same six selected target/epoch pairs were queried with both Earth center 399 and Solar System Barycenter 0 as observers, at three half-steps each. Changing the observer changes the geometry and does not by itself isolate one correction. For Moon year 100, `Q45 derivative − Q46` in RA·cos(dec) was +0.186410″/hour for Earth and −0.000748727″/hour for the barycenter at the 0.002-day half-step. But for Mercury year 0 it was +0.00250142″/hour for Earth and **+0.01178984″/hour** for the barycenter. Therefore zero observer barycentric velocity does not generally eliminate the discrepancy; target emission-time variation can remain.

For that barycentric Mercury sample, the independent Rust light-time solver was run with observer 0 and target 199. The Rust diagnostic includes only light time, no stellar aberration or solar deflection, and projects rates on its light-time direction; the Horizons direction includes its stated corrections. Values in ″/hour:

| Quantity | RA·cos(dec) | declination |
| --- | ---: | ---: |
| Horizons Q45, centered difference at ±0.002 d | 480.40618984 | 109.75746979 |
| Rust light-time state, including `dLT/dt` | 480.40618962 | 109.75747776 |
| Horizons Q46 | 480.39440000 | 109.75480000 |
| Rust diagnostic omitting `dLT/dt` | 480.39435436 | 109.75477378 |

Here `dLT/dt = −2.46365516e−5` s/s. The complete and incomplete light-time rates track Q45 and Q46 respectively to printed-precision scale for this case. This isolates the retarded-time contribution numerically; solar deflection, model differences, timestamp and output precision remain possible smaller effects. The other terms are supported by the geocentric component experiment, not uniquely isolated by this single Mercury comparison. The official documentation's broad description does not settle the exact internal derivative convention. JPL confirmation or published algorithm details are needed before asserting a definitive Horizons implementation defect or intent.

## Prespecified modern Mercury holdout — initial comparison fails

Five new barycentric Mercury dates (July 1, 12:00 TDB, 1991, 2003, 2017, 2023 and 2031) were specified after examining the year-0 case. Before querying, both the light-time-only Rust rate versus the Q45 derivative and the hybrid versus Q46 were assigned a 0.001″/hour per-component diagnostic limit. At the ±0.002-day half-step, the former comparison **failed 5/5**; the latter **failed 2/5**. The maximum absolute component discrepancies were respectively **0.0126034″/hour** and **0.00172367″/hour**, both in RA·cos(dec), year 2031. This falsifies treating the stripped light-time-only model as a generally sufficient predictor of apparent Q45 or Q46. The SSB lies close to the Sun and Horizons explicitly reports solar light bending; this observation is not by itself a full attribution of the residual.

To identify the missing stage, a separately predeclared follow-up requested Horizons Q1 astrometric and Q45 apparent directions at the same five dates and SSB observer, plus Q46. Horizons Q1 has down-leg light time but not the apparent corrections. At ±0.002 day, **all 5/5** Q1 centered differences agreed with the Rust light-time-only derivative within the separate 0.001″/hour component limit; the largest discrepancy was **0.000216937″/hour**. Q45 and Q1 themselves differed materially: in year 2031 their RA·cos(dec) derivatives were **476.244002** and **476.256493″/hour**, respectively. The omitted apparent corrections therefore explain the failed simplified comparison in that case; the measurement does not uniquely assign each correction or settle JPL's algorithm. The initial failures remain failures, not reclassified passes. At the SSB, zero observer barycentric velocity also does not remove light deflection or make the observer's proximity to the Sun a normal viewing geometry.

## Reproduction and retained evidence

From the repository root, provide an already built Node addon, a legally obtained classic DE441, its SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`, and new output paths. The commands perform network requests where noted, and refuse existing output paths. The Q46 grid command writes its report then exits nonzero when the diagnostic limit is exceeded.

```text
cargo build --release --offline -p virgostars-ephemeris-core --example audit_retarded_velocity
cargo build --offline -p virgostars-ephemeris-core --example diagnose_icrf_rate_components
node bindings/node/scripts/compare-horizons-icrf-rates.cjs <kernel> <sha256> <new-q46.json> 0-3000-step-50
node bindings/node/scripts/compare-horizons-rates-by-position.cjs <kernel> <sha256> <new-q45.json> 0-3000-step-50
node bindings/node/scripts/audit-horizons-rate-components.cjs <kernel> <sha256> <new-q46.json> <q46-sha256> <component-exe> <new-components.json>
node bindings/node/scripts/diagnose-horizons-rate-consistency.cjs <kernel> <sha256> <new-six.json> 0.002
node bindings/node/scripts/audit-q46-observer-centers.cjs <new-six.json> <new-observers.json>
node bindings/node/scripts/audit-q46-retarded-velocity.cjs <kernel> <retarded-exe> <new-observers.json> <new-retarded.json>
node bindings/node/scripts/audit-q46-observer-centers.cjs <mercury-holdout-input.json> <new-holdout-observers.json>
node bindings/node/scripts/audit-q46-retarded-velocity.cjs <kernel> <retarded-exe> <new-holdout-observers.json> <new-holdout-rates.json>
node bindings/node/scripts/audit-q46-astrometric.cjs <mercury-holdout-input.json> <new-holdout-rates.json> <new-q1-report.json>
```

Repeat the six-case command with separate output names and half-steps 0.0005 and 0.01 for the step sensitivity check. The holdout input's five JD TDB values are `2448439, 2452822, 2457936, 2460127, 2463049`; it is a JSON object with `rows` containing name `Mercury`, year and `jdTdb` for each date. The exploratory scripts use the existing core and coordinate-only rate conversion; they do not install a second astronomy engine. Reports and full raw responses remain on the owner's machine, not in the repository. The kernel hash is checked before and after the local comparisons. For this execution, `q46-components-610.json` SHA-256 was `b57d2dc2f54b83afaa9a24ce84eb3abb15bd0b21585177abc8898edf71143d3f`; `q46-retarded-final.json` was `43630a2fe1a6aa45c6bd4ad951895143ee9f8bce71ab523abff0c85ec31c7ff5`; `q46-mercury-holdout-rates.json` was `bd2de0a550255d54a7749eb7474df93f8335e3bee68386532c43915981b408d5`; `q46-mercury-holdout-q1.json` was `d32fe2ae223c35de6926d64e694170d3b203ab45c6fe2feef168bffae33a03d3`. The exact requests and output contain timestamps, so new report hashes will differ.

Local Windows verification: `cargo test --workspace --offline` passed 508 tests, with 31 ignored; the explicitly invoked four real-DE441 apparent tests passed 4/4. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, and Node syntax checks for the three added orchestration scripts passed. The raw network comparisons and the Rust example were executed as documented; the failed diagnostic thresholds are reported above.

The limits of the apparent model, ecliptic-of-date speeds, radial speeds, near-solar behavior and broader numerical validation remain open. No app activation follows from this investigation.
