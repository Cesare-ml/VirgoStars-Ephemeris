# Historical ΔT model and uncertainty audit — 2026-11-18

Status: **experimental; the Espenak–Meeus 2006 model must not be promoted as a validated historical-time default.** The numerical comparisons and diagnostic thresholds were specified in the [plan](DE441_DELTAT_REFERENCE_PLAN_2026-11-18.md) before the comparison. Base `0cc404167218a0159d9a664c8171b652c4d9f268` (PR #637). This audit changes no app and introduces no new automatic ΔT or civil-time policy.

## Model and independence

The actual Rust `espenak_meeus_2006_long_range` returns TT−UT1 in seconds at a caller-supplied decimal year. It implements the [NASA 2006 piecewise fits and projections](https://eclipse.gsfc.nasa.gov/SEcat5/deltatpoly.html), including a 2005–2050 forecast published in 2006. It supplies **no** uncertainty. Year 0 is 1 BCE; the local dataset uses proleptic Gregorian calendar when defining calendar points. NASA's source values before 1600 derive from historical eclipse/occultation reconstructions, and later published reconstructions and NASA/JPL model values can share observations. Agreement between those models is not independent verification of physical Earth rotation.

The independent evidence used here is more specific: the [USNO observed monthly ΔT series](https://maia.usno.navy.mil/products/deltaT), especially observations after the 2006 forecast was made; and the 2026 [Hayakawa, Sôma and Li eclipse analysis](https://doi.org/10.1093/mnras/stag656), based partly on previously unused historical records and DE441. Its limits depend on interpreting reports, sites and ephemerides; they are not independent direct measurements of a continuously varying function or stated confidence intervals. The 1361 record is reanalysed, and the 1514/1542/1575 constraints add material. The independent records and all older reconstructions may still have dependencies.

## Findings

| Regime / reference | Result and proper interpretation |
| --- | --- |
| 0–1600: NASA 2004-based history and Horizons | On 22 July 1 TT samples from astronomical 0–1900 plus 1950/1961, old model minus Horizons ranges up to **193.773 s absolute**, at 1300. Both reconstructions can share historical evidence. JPL Q30 before 1962 approximates TT−UT1; JPL does not retain the millisecond TT/TDB distinction. At later dates Q30 switches to TDB−UTC and is **not** ΔT. These are model differences, not uncertainty bounds. |
| 1361, 1514, 1542, 1575 eclipse records | At the NASA convention's midpoint of the paper's eclipse month, **3 of 4** model values fall in at least one authors' stated interval. For 1514 August the model gives **183.569 s**, below the combined conditional interval **277–890 s** by **93.431 s**. The model varies from 183.610 to 183.528 s over that month, so month selection does not resolve the discrepancy. The 1361 record has two disjoint admissible ranges; both were preserved. This is conditional evidence, not a proof that either value is historical truth. |
| 1657–1971.5: USNO historic | 630 half-year rows compared; maximum absolute difference **12.681 s** at 1686.5. Of 629 rows with nonzero published `error`, **283** differ from the model by more than that number. The file does not define a confidence level in its header, and the compared reconstructions may overlap; the count is not an independent p-value or a calibration of the model's uncertainty. |
| 1973–2004: USNO monthly | **383/383** monthly samples are within the predeclared 0.5 s diagnostic limit; maximum absolute residual **0.123 s**. This describes the sampled published fit, not every instant. |
| 2005–2026: USNO monthly | **176/262** samples exceed the predeclared 1 s retrospective diagnostic limit. First exceedance is 2012-03-01, +1.009 s. On 2026-10-01, model **75.536 s**, USNO **69.2065 s**, residual **+6.330 s**. USNO reports lower precision than daily EOP products. The cutoff is not a physical error bar or a product acceptance requirement. |
| 2027–2300: forecasts | There are no future observations against which to validate the forecast. The locally captured USNO long-term prediction file has an uncertain forecast vintage; e.g. its 2027.0 value is 69.14 s vs the 2006 model's 75.693 s. This comparison neither proves either forecast nor authorizes selecting its `Error` column as the application's future uncertainty. |

The 1514 eclipse is conventionally dated **20 August Julian**; NASA's [eclipse catalogue](https://eclipse.gsfc.nasa.gov/SEsearch/SEdata.php?Ecl=15140820) documents use of the Julian calendar before 1582. The comparison uses the published month-level polynomial input, not an assertion about historical civil calendars. For all four eclipse rows the full-month numerical probes and source intervals are in the machine-readable report.

For illustration, the [NASA uncertainty discussion](https://eclipse.gsfc.nasa.gov/LEcat5/uncertainty.html) reports source-era standard errors near year 0, 500, 1000 and 1500 of approximately **260, 140, 55 and 20 seconds**, respectively. They describe older reconstructions and their assumptions, not newly calibrated guarantees for this Rust model; applying a constant ±60 seconds throughout the whole interval is not justified. For the future the same NASA discussion describes a Huber drift/fluctuation model (N years after 2005, M=2500 years, Q=0.058 ms²/year), giving illustrative σ ≈ **47.9 s in 2100**, **143.6 s in 2200**, **272.1 s in 2300**, and **612.2 s in 2500** (the source rounds the latter to 612 s). These are model-dependent extrapolations, not observations, established coverage or a validated uncertainty band for the 2006 polynomial. Do not backfill future values from Horizons Q30, which uses UTC and frozen/predicted EOP outside its prediction horizon.

## Real-DE441 Moon sensitivity examples

After the reference comparison, seven exploratory rows were computed at fixed UT1 with the existing **experimental** multipack apparent ecliptic-of-date provider and local packs. All ten package hashes and sizes were verified before and after. TT was computed from caller-supplied ΔT; the experimental geocentric TDB correction and long-term pole/nutation frame from #637 were used. Sun, Moon and Earth states come from the same DE441 data. Values below show longitude change from nominal; these are samples, **not angular uncertainty bounds**, since intermediate extrema, model and frame uncertainty, calendar and locality are not included.

| Illustrative epoch | ΔT perturbation | Moon longitude change at ± perturbation |
| --- | ---: | ---: |
| Year 0, July | ±260 s | approximately ±2.131 arcmin |
| Year 500, July | ±140 s | approximately ±1.432 arcmin |
| Year 1000, July | ±55 s | approximately ±0.476 arcmin |
| Year 1500, July | ±20 s | approximately ±0.169 arcmin |

Near the 1514 eclipse (chosen fixed UT1 JD **2274277.68**, an illustrative instant close to the event), using the model's 183.569 s gives Moon longitude 155.635301°. Using 277 s gives +0.978 arcmin, and using 890 s gives +7.396 arcmin, relative to that nominal case. These are separate conditional values, not a symmetric ± interval and not a reconstruction of an individual's local clock or of eclipse visibility. The 0/500/1000/1500 σ values are tabulated at whole years and used illustratively near July, not interpolated as per-date observations.

## Reproducibility and evidence

Windows 11, Rust `1.99.1`, Cargo `1.99.1`, pyerfa `2.0.1.5`. The local Rust executable evaluates the existing model; Python only orchestrates sources, date fractions, comparison and reporting. JPL observer query uses Sun (10), geocenter (399), discrete **TT** Julian dates, Q30 and DE441 source. Proleptic Gregorian July 1 noon was used to generate JD; Horizons displays mixed Julian/Gregorian dates, but epoch comparison is in JD. The NASA model is evaluated at the corresponding approximate decimal year; TT vs UT1 differs by up to hours in early samples and hence produces a small decimal-year mismatch, not a source of independent accuracy. No JPL planetary-position comparison is claimed in the ΔT audit.

```powershell
cargo fmt --all -- --check
cargo test --workspace --offline --quiet
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo build --release --offline -p virgostars-ephemeris-core --example audit_delta_t_reference --example audit_delta_t_sensitivity
python scripts/audit_deltat_observations.py <reference-exe> <usno-snapshot-dir> <new-report.json>
python scripts/audit_deltat_sensitivity.py <reference-report.json> <manifest.json> <pack-dir> <sensitivity-exe> <new-report.json>
```

The reference audit intentionally returns exit code **1** after writing results because the predeclared 2005+ threshold and one eclipse interval fail. It requires network access to Horizons and pyerfa. The sensitivity audit exits 0. Rust workspace: **524 passed, 0 failed, 31 ignored**; fmt and Clippy passed. The DE441 manifest SHA-256 is `a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a`; no ephemeris files are included or distributed.

Owner-local evidence under `C:\Users\Windows11\.codex\visualizations\2026\11\18\01a1ec76-743f-79f2-a39d-b5623ef17cab\science-evidence-20261118`:

- `deltat-usno-sources-02/historic_deltat.data`: SHA-256 `9f43514119060601a00624a5d9287a91b0b1f5e12bec5209bbabda90b392b70b`; `deltat.data`: `7396fcfdad33856b5242f288887678d75f3803494fa79394bc02f37eb99efc1e`; `deltat.preds`: `5d864fddd30b2c64d2a86d3debbb25604eb5de44370c96bccf2abd5463f3db08`. Snapshots were saved as UTF-8 text; response metadata reported 2026-10-15 for all three, which does not prove prediction issue date.
- `deltat-observation-run-01.json`: SHA-256 `1b73ed342123d3b471c55ec10e4569026a021c73496256988cb95eb5a6ebd205`; contains all rows, source hashes, query URL and raw Horizons response. Latest included observation 2026-10-01; no row after as-of date 2026-11-18 is treated as observed.
- `deltat-sensitivity-run-01.json`: SHA-256 `018551181c4d9788b828d1eec797d21918a45b62fcbb26454d62a955212df524`; contains the input epochs, scenario values and sampled Moon position fields. The reference and sensitivity executables have SHA-256 `9e2ffb87d211ce0cab15a654d2c479634a7a4f49161c2efefee8c53ff16fb993` and `ce9d00887a6edb2534a388347a5a797eef6dfbb1ce48396cdd3e855d095396bc`.

Before adopting any historical default, choose a reviewed provenance-aware model by period, validate modern EOP/leap-second inputs, characterize confidence and out-of-sample behavior for each regime, preserve genuinely unknown cases and civil-time uncertainty, and repeat independent validation. Long-term orientation, houses and Android integration remain separate gates. Do not reinterpret this audit as validation of the entire 0–2300 span or future truth.
