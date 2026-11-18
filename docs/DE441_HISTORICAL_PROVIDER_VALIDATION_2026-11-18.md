# Experimental historical UT1 provider validation — 2026-11-18

Status: **EXPERIMENTAL; the exploratory ERFA time threshold was not met.**
Base: `6b71c28a1a04bef075c94842368ba4f6f93ccab7` (PR #631).
The prior [plan](DE441_HISTORICAL_PROVIDER_PLAN_2026-11-18.md) specified the tests and thresholds before running them.

## Contract and scope

The new Rust entry point uses the existing pole-based apparent pipeline with a per-epoch provider. The caller supplies JD UT1, TT−UT1 seconds, a nonnegative perturbation, and a bounded source label. It returns lower, nominal and upper samples only when all succeed, otherwise preserving the failing sample and typed underlying error. The label is an assertion, not checked provenance. The samples are sensitivity examples, not an uncertainty bound or confidence interval. UT1 is not UTC or local civil time.

No Mobile changes or new implicit time policy were made. This work does not remove the separate `house_angles` orientation limit or expand Mobile's existing civil-time range. Long-range Vondrák orientation and extrapolated nutation remain experimental.

## Inputs, integrity and environment

Windows 11; `cargo 1.99.1 (5f94df478 2026-08-27)`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, pyerfa 2.0.1.5. All input sizes and SHA-256 were checked before **and** after the numerical run. Input files were not modified or copied to the output. Source: `D:\GitHub\virgo-stars\database\effemeridi\astrodienst\de441.eph`, 2,788,676,624 bytes, SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`. Manifest: `D:\VirgoStars-DE441-Mobile\packs\de441-mobile-manifest-v1.json`, SHA-256 `a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a`; 10 package hashes and byte counts checked against that manifest. Example core: 46,599,968 bytes, SHA-256 `863c83229749d12751320447da01ddd9e72e813c7512723b8cab905ec69ab21f`. Input identities prove data consistency, not independent scientific correctness or distribution rights.

Evidence is owner-local under `C:\Users\Windows11\.codex\visualizations\2026\11\18\01a1ec76-743f-79f2-a39d-b5623ef17cab\science-evidence-20261118\historical-provider-run-02`; `report.json` SHA-256 `e968a70b320865106ebcb683980625370a7ea33a0e2d60cc4c59b8e6bb897cc9`. It records all 300 observations, 372 time residuals, actual requests and raw JPL responses. `stdout.csv` SHA-256 `9681bd9d9989a3d598f8b01244a9ae9ec43a6948ca26bd41939e23a466a2c845`. Binary SHA-256 `563837216b92b40d10b9374242b105459060b8e4cdfe705bb2be88d3c0f7aa38`. The preceding run is retained separately as `historical-provider-run-01`.

## Results

* **Rust:** `cargo fmt --all -- --check` passed; `cargo test --workspace --offline --quiet`: 520 passed, 0 failed, 31 ignored; `cargo clippy --workspace --all-targets --offline -- -D warnings` passed. The filesystem could not create hard links for incremental caches; Rust copied cache files instead. The four new provider integration tests passed within the workspace suite. Release audit example compiled successfully.
* **Provider against full local source:** all 300 sampled cases (10 packs × 10 targets × three Delta-T samples) matched exactly in all six reported `f64` values. Predeclared tolerances: 1e-8 degree or degree/day, 1e-12 AU or AU/day. This is same-ephemeris consistency, not independent physical validation. For each of nine transitions, an absent next file produced the expected typed upper-sample error. One corrupt synthetic file and one outside-coverage epoch produced expected typed errors. The scenario adapter does not hide those failures.
* **JPL + ERFA:** 300/300 sampled longitudes, latitudes and ranges satisfied the predeclared 1 arcsecond per angle and 1e-9 AU limits. Maximum absolute residuals: longitude 0.0012854243 arcsec, latitude 0.0000029804 arcsec, range 1.49204e-11 AU. Targets are Sun, Moon, Mercury, Venus and Mars–Pluto barycenters; observer is Earth center `500@399`; time is TT; Horizons Q45 apparent ICRF and Q20 range, AIRLESS (no atmospheric refraction); ERFA Vondrák long-term precession with full IAU 2000A nutation rotates the direction to ecliptic of date. The requests confirm DE441 as source. This checks sampled implementations/frames against independent software with the **same DE441 model**, not independent knowledge of planetary positions. No speed comparison is claimed here. Requested Horizons epochs are rounded to 9 JD decimals; reference and Rust model approximations differ.
* **ERFA time comparison — FAILED exploratory limit:** 369/372 represented TDB−TT values fell within 100 microseconds; 3 exceeded it. Worst absolute residual was 123.109069 microseconds at astronomical year 75, October. Samples comprise four seasons every 25 years from astronomical year 0 through 2300. Inputs used a fixed illustrative 64-second TT−UT1 solely for this model comparison. Geocentric `erfa.dtdb` is the independent software reference, not a historical measurement.
* **Post-hoc diagnosis, not a new acceptance criterion:** at the worst epoch the existing leading-term TDB−TT model differs from ERFA by +115.096361 microseconds, and adding it to a single `f64` JD contributes +8.012709 microseconds. Across the samples, maximum absolute model residual is 115.096361 microseconds and maximum absolute representation contribution is 20.054625 microseconds, potentially at different epochs. The original 3 failures and 100-microsecond criterion remain unchanged. The leading term has not been upgraded or proven adequate throughout the historical interval.

The provider examples use the existing [Espenak–Meeus polynomial](https://eclipse.gsfc.nasa.gov/SEcat5/deltatpoly.html) as an explicitly chosen nominal model, and ±60 seconds as an illustrative perturbation only. This is not a statement of measured Delta-T uncertainty. The [ERFA `dtdb` implementation](https://github.com/liberfa/erfa/blob/master/src/dtdb.c) provides a distinct time-model comparison, not evidence of ancient UTC, actual Earth rotation, local time, longitude, timezone, calendar, or clock accuracy.

## Reproduction and open work

```powershell
cargo fmt --all -- --check
cargo test --workspace --offline --quiet
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo build --release --offline -p virgostars-ephemeris-core --example audit_historical_ut1_provider
python scripts/audit_historical_ut1_provider.py <source.eph> <manifest.json> <pack-dir> <audit-exe> <new-output-dir>
```

The Python comparison requires `pyerfa`, dependencies of the existing audit helpers, and access to JPL Horizons. The output directory must be new and separate from data. The command intentionally returns nonzero while the original ERFA threshold fails, after writing all evidence. Before using this for historical birth charts, define an evidenced Delta-T/uncertainty policy and civil calendar, timezone, local-time and UT1 conventions; resolve or explicitly accept the time-model accuracy gap; validate long-range orientation and the house/angles path independently; then integrate and test the Mobile contract. Do not infer user-facing historical coverage solely from DE441 package coverage. Distribution rights remain `review_pending`.
