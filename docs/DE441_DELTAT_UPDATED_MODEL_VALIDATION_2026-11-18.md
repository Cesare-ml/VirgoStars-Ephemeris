# Updated ΔT candidates across the DE441 packs — 2026-11-18

**Outcome: partial experimental validation, not a certified 0–2300 policy.** The 2021 Morrison–Stephenson–Hohenkerk–Zawilski S5 spline is a documented historical candidate; observed USNO data are the stronger option for their measured interval. The independent 1514 constraint is still missed; the 1973 transition is discontinuous; future ΔT cannot be validated against observations. There is no implicit switching, extrapolation, uncertainty model or app change. This branch is based on `781e08ca2b2ac350610052b8b5f5109e92b22454` (PR #639). See the predeclared [plan](DE441_DELTAT_UPDATED_MODEL_PLAN_2026-11-18.md) and [previous 2006-model audit](DE441_DELTAT_REFERENCE_VALIDATION_2026-11-18.md).

## Candidate by period

| Period, astronomical year | Candidate / result | Decision and limits |
| --- | --- | --- |
| 0–1973, especially 0–1600 | [Morrison et al. 2021 S5](https://doi.org/10.1098/rspa.2020.0776) piecewise cubic, fitted to historical observations. The supplied source table covers −720.0 to 2019.0. | Experimental candidate only. Uncertainty is not supplied by S5 and the 2026 1514 constraint fails. USNO historic reconstructions cover 1657–1984.5 as a separate alternative, not direct atomic observations throughout. |
| 1973-02-01–2026-10-01 | Caller-supplied [USNO monthly observations](https://maia.usno.navy.mil/products/deltaT), with existing `DeltaTTable`. | Useful experimental observed-data path; requires a verified and maintained snapshot. Monthly interpolation is sampled below; missing error values remain unknown. Input JDs in the existing table are treated as UT1 although published monthly dates are UTC; subsecond UT1−UTC and interpolation assumptions remain. |
| After 2026-10-01 through 2033 | The local USNO prediction snapshot contains later values, marked `Predicted`, but its forecast issue date has not been established. | Do not treat the snapshot as current or its reported error as a validated bound. No automatic future-model selection. |
| 2033–2300 | Long-range ΔT and uncertainty models are extrapolations; e.g. NASA's older Huber drift/fluctuation estimate in the previous audit. | **Not physically validated by future observations.** A forecast, its as-of date, assumptions and uncertainty need a separate policy; 2006 Espenak–Meeus is demonstrably outdated in modern retrospective tests. |

The new Rust `DeltaTSpline` evaluates normalized cubic segments explicitly supplied by a caller; it checks finite, ordered contiguous segments, a 5 ms knot-integrity limit and range, with no extrapolation. Coefficients are **not bundled**; the extractor reads an owner-local published PDF. Evaluating a syntactically valid table does not authenticate its scientific provenance or provide uncertainty. The source publication and supplementary material are separately licensed; their reuse terms were not established for redistribution here, so neither the PDF nor coefficient data is committed. The code is separate from any Swiss Ephemeris implementation.

## Historical model checks

The [publisher supplement S5/S6](https://s3-eu-west-1.amazonaws.com/pstorage-rs-4828782598/26513566/rspa20200776_si_001.pdf) supplies the 58 coefficient rows and independently tabulated rounded values. All **11/11** selected S6 values agree with the S5 evaluator at their published rounding precision. All **57/57** internal knots satisfy the predeclared 5 ms numerical gate: worst sampled jump **1.000012 ms** at year −100, consistent with rounded coefficients. This checks transcription and evaluation, not independent Earth-rotation accuracy. S6 contains apparent sign inconsistencies: its coarse 1900 row says +2 s while the detailed row/S5 give −1.98 s; its detailed 1880 row says +3.21 s while S5 gives −3.210 s. Those inconsistent entries were not counted as positive fixtures.

At the same month midpoints as the prior audit, the 2021 spline is inside 3/4 conditional intervals from [Hayakawa, Sôma and Li (2026)](https://doi.org/10.1093/mnras/stag656):

| Eclipse year | Candidate TT−UT1 | Published admissible ΔT | Result |
| --- | ---: | --- | --- |
| 1361 | 556.833 s | −408–601 s **or** 1865–18532 s | inside the first interval |
| 1514 | 264.050 s | 277–890 s | **outside**, 12.950 s below |
| 1542 | 210.205 s | −328–332 s | inside |
| 1575 | 150.191 s | −1762–1091 s | inside |

These are constraints conditional on historical reports, site identification and DE441-based geometry, not probability intervals or direct verification of a person's clock. The new candidate is closer to the 1514 lower limit than the old 2006 fit (183.569 s), but it does **not** satisfy that condition.

Against 630 USNO historic half-year entries from 1657–1971.5, 464 of 629 with a nonzero published `error` differ from the spline by more than that reported number; the maximum absolute difference is **9.032 s** at 1720.0. These sources may share observations; the USNO file header does not establish confidence semantics, and an out-of-column count is not a statistical rejection rate. Different reconstruction vintages and fit choices matter. At 22 pre-1962 sampled epochs, the maximum S5 vs Horizons Q30 difference is **40.309 s**, versus 193.773 s for the old fit on the same points. JPL and the spline share reconstruction lineage; this is not independent physical accuracy or an uncertainty estimate. Before 1962 Q30 approximates TT−UT1; after 1962 it means TDB−UTC and is not used here.

## Modern data and uncertainty

For 644 15th-of-month 00:00 UTC epochs from 1973-02-15 through 2026-09-15, the existing Rust monthly interpolation was compared with [USNO finals2000A](https://maia.usno.navy.mil/ser7/finals2000A.all) Bulletin A observed (`I`) UT1−UTC, using `TT−UT1 = 32.184 + (TAI−UTC) − (UT1−UTC)` and ERFA's leap-second data; [IERS Bulletin C 72](https://datacenter.iers.org/data/html/bulletinc-072.html) confirms 37 s in 2017–2026. **644/644** differences meet the predeclared 0.05 s diagnostic threshold; worst absolute difference **0.005357 s** at 1997-11-15. These sources may share underlying EOP observations. The test checks sampled monthly interpolation and file interpretation, not continuous accuracy, interpolation-error bounds or an independent measurement campaign. It does not turn UTC into UT1: at these instants the legacy reader is evaluated at the same numerical JD, and the UTC/UT1 argument difference and its effect are not separately bounded here.

At 1973-02-01 the spline gives **43.243984 s** and the first USNO monthly value is **43.4724 s**, a **0.228416 s** step. The historical USNO decimal-year conversion also uses a documented 365.25-day convention, distinct from Gregorian year fractions. No splice or interpolation between those different policies is automatically selected.

An implementation defect was corrected in `DeltaTTable`: formerly, interpolation between one endpoint with an unknown source `error` and another with a reported error returned the latter as if it described the interpolation. It now returns `None` unless **both** endpoint errors are known; exact points retain their own source value. Malformed, nonfinite and negative error entries now fail parsing. This changes the public `uncertainty_seconds` option for mixed-known/unknown intervals; consumers must handle `None`. Even where two source error entries exist, their interpolation is not an independently justified confidence interval or total interpolation-error bound. The old checkpoint's “1σ” wording was explicitly corrected. On the actual snapshot, the day after the last observation is `Predicted` with `unknown` error; the 2027-01-01 source prediction is `Predicted` with source `error = 0.327 s`, whose confidence meaning and vintage are not established.

## Illustrative real-pack sensitivity

After the model comparisons, at fixed UT1 JD 2274277.68 near the 1514 eclipse, the existing experimental DE441 multipack pipeline gave apparent geocentric ecliptic Moon longitude 155.635301° for the old 2006 value, 155.649345° for the S5 candidate, and 155.651604° for the paper's 277 s conditional lower value. The S5 result is **+0.843 arcmin** relative to the old fit; the conditional lower value is **+0.136 arcmin** relative to S5. All ten pack sizes/hashes were verified before and after. This is the same DE441 and experimental orientation, not independent physical orbit validation, a confidence interval, a bound on positions, or an app test.

## Verification and evidence

Windows 11; Rust and Cargo 1.99.1; pyerfa 2.0.1.5; pypdf for the owner-local PDF extraction. `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets --offline -- -D warnings` pass. `cargo test --workspace --offline --quiet`: **527 passed, 0 failed, 31 ignored**. The previously ignored real-USNO table test was also run separately with `USNO_DELTAT_DIR` and passed **1/1**. Initial filesystem hard-link warnings indicate cache copying, not test failures.

```powershell
cargo build --release --offline -p virgostars-ephemeris-core --example audit_delta_t_candidate
python scripts/extract_morrison_s5.py <official-supplement.pdf> <new-rows.txt>
python scripts/audit_updated_deltat_models.py <candidate-exe> <rows.txt> <usno-dir> <finals2000A.all> <prior-report.json> <new-report.json>
python scripts/audit_deltat_candidate_moon.py <new-report.json> <prior-report.json> <manifest.json> <pack-dir> <sensitivity-exe> <new-moon-report.json>
```

The reference audit deliberately exits **1** after writing the report because the 1514 conditional interval fails. The extraction and Moon sensitivity audits exit 0. All inputs are read only, hashed before and after; no kernels or third-party source tables are committed. The exact JPL query and USNO snapshot provenance are retained in the earlier report. Local files under `C:\Users\Windows11\.codex\visualizations\2026\11\18\01a1ec76-743f-79f2-a39d-b5623ef17cab\science-evidence-20261118`:

- `morrison-2021-supplement.pdf`: SHA-256 `e30601b2c9ac2169f923fdaa6c9bc76628b393b36ad3a0e18385111166e24255`; extracted `morrison-2021-s5-extracted-02.txt`: `bd953d93b5f2b816c9b699ceb5572d2cc82b624009880d261a2b2ef5a1c7050c`. The extractor checks 58 rows, full endpoints and the PDF hash; the PDF parser's rotated-text warning pertains to page furniture, and the coefficient rows were checked against S5.
- `usno-finals2000A-20261118.txt`: SHA-256 `51cb9676d9c4775c4d344b328bc3eb1ac3940b089ce4b659ccac883637d09dd6`. USNO monthly/historic/prediction source hashes and vintage caveats are in the previous report. DE441 manifest SHA-256 `a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a`.
- `deltat-updated-model-run-04.json`: SHA-256 `8c5d0007411d0b65ce21e9d2699a21123e1f7efcc6fac39c2db1a575f1bb2abf`; `deltat-updated-moon-run-02.json`: `ef4cb56bc26632cc7aea20aa58e7fe1cea0e6d8919fca1ff0195482047dabe26`. The reference executable SHA-256 is `e1ba5d330cf0aed581f02fddc42546a61bdffd9a22c4d0b757f810bef535b775`.

Next work: obtain a current, dated future prediction and its documented uncertainty; resolve historical evidence and accuracy limits including 1514; decide an explicitly reviewed source selection and splice contract; validate the long-range frame, houses and any civil-time conversion separately. No result here authorizes activating the candidate in the app or assuming a universal ±60 s.
