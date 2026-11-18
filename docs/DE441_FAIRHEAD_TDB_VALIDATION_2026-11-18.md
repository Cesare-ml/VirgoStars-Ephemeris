# Geocentric historical TDB−TT correction — 2026-11-18

Status: **experimental, opt-in**. The unmerged branch is based on `b4012c7d05e41743129c5e9f70087aa57c491ec8` (PR #635). This report supplements, rather than rewrites, the [previous failure report](DE441_HISTORICAL_PROVIDER_VALIDATION_2026-11-18.md). The [validation plan](DE441_FAIRHEAD_TDB_VALIDATION_PLAN_2026-11-18.md) set the thresholds and sample design before these runs.

## Change and scientific contract

The earlier provider converted caller-supplied UT1 and Delta T to TT, then used a single sinusoidal term for geocentric TDB−TT. In three of 372 seasonal samples that represented result differed from ERFA by over 100 µs; the largest residual was 123.109 µs. The main cause was the one-term model, not solely JD rounding.

The experimental multipack UT1 scenario path now evaluates the complete 787-term Fairhead–Bretagnon series and JPL mass adjustment using coefficient data from [ERFA 2.0.1 `dtdb`](https://github.com/liberfa/erfa/blob/v2.0.1/src/dtdb.c), geocentric observer (`u=v=0`). The published model formally takes TDB, and—as ERFA documents—we substitute TT; no topocentric contribution is included. TT calculated from caller UT1 and TT−UT1 is passed directly to date-frame rotation instead of reconstructed using the old inverse one-term approximation. The legacy single-reader and other pre-existing conversion APIs continue using their existing model.

The new converter accepts TT JD **1,721,058.5 through 2,561,483.5 inclusive**, an engineering guard extending one day beyond the pack range; nonfinite or outside-guard inputs fail explicitly. TT, UT1, TDB, Delta T and its perturbation remain explicit; no UTC, timezone, historical calendar, civil-time or Delta-T model is inferred. The scenario values remain samples, not uncertainty bounds. Separate-f64 Julian dates still quantize both the UT1→TT addition and TT→TDB addition; the residual reported below examines the latter difference at the already represented TT. No guarantee of total elapsed-time precision is implied.

The 787 coefficient triples carry NumFOCUS copyright and the separate BSD-3 terms already included in `third_party/ERFA_LICENSE.txt`; `THIRD_PARTY_NOTICES.md` records their source. Source `dtdb.c` downloaded from the tagged URL on 2026-11-18, SHA-256 `228bd7c7c1c88dda44c29a076f289c968f64d77a2dc389138b03ab5d1d442105`. This Rust series and ERFA **share the same model and coefficient data**. Agreement is software/model-conformance evidence, not independent physical validation of the model.

## ERFA numerical comparison

Reference: pyerfa 2.0.1.5 `dtdb`, geocenter, JD TT in the model argument; an illustrative fixed TT−UT1 of 64 s. Astronomical year numbering is used (year 0 = 1 BCE), proleptic Gregorian. Thresholds were declared before testing: raw correction vs ERFA ≤1 ns; represented `(JD_TDB−JD_TT) × 86400` vs ERFA ≤25 µs. The original 100 µs represented-value gate was also retained.

| Samples | Count | Maximum raw model residual | Maximum represented residual | Failures |
| --- | ---: | ---: | ---: | ---: |
| Seasonal, every 25 years, 0–2300 | 372 | 4.988e-18 s | 20.103 µs | 0 |
| Monthly holdout, every year, 0–2300 | 27,612 | 4.999e-18 s | 20.117 µs | 0 |
| Eleven logical boundaries, −1/0/+1 s | 33 | 4.771e-18 s | 17.715 µs | 0 |
| Total | 28,017 | 4.999e-18 s | 20.117 µs | 0 |

The largest monthly represented residual was at UT1 JD 2257250.0, TT JD 2257250.000740741, astronomical year 1468 January. The raw same-model residual there was −1.19e-18 s, while `f64` representation contributed +20.116 µs. The 1 ns figure is a **conformance threshold**, not a physical error estimate. The original worst epoch (year 75 October, UT1 JD 1748727.0) is retained as a Rust regression: the legacy result exceeds 100 µs and the new represented result is below 25 µs.

ERFA describes its geocentric series as a model; its cited ±3 ns comparison to DE405-based time ephemerides applies to 1950–2050 and must not be extrapolated into ancient epochs. Long-range model uncertainty, actual historical Earth rotation and Delta T remain separate open problems. JD precision does not determine their accuracy.

## Real packs and apparent-direction checks

All ten pack artifacts, source and manifest were hashed and sized before and after the run. Source SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`, 2,788,676,624 bytes; manifest SHA-256 `a4a204169555ee4ec01d5b3f25ce331a44ff6c9fd273d7f812cfa245e6ef6b7a`. No DE441 data were copied or distributed. Rights remain `review_pending`.

At each pack midpoint, three illustrative Espenak–Meeus Delta-T values (nominal and ±60 s sensitivity, **not** measured uncertainty) were evaluated for Sun, Moon, Mercury, Venus and six Mars–Pluto system barycenters. **300/300** provider results matched the full-source reader in all six fields, at identical TT/TDB, with numerical differences zero in these samples. Same source and same pipeline mean this is an input-selection consistency check. Nine missing upper-neighbor cases, one corrupt synthetic file and one outside-pack case produced their expected typed errors. New model-range failures are separately tested.

The same 300 apparent directions/ranges were compared with JPL Horizons Q45/Q20, center Earth `500@399`, ICRF apparent, AIRLESS, TT, using ERFA Vondrák long-term precession and full IAU 2000A nutation to rotate the JPL direction to ecliptic of date. Both use DE441. Maximum residuals: **0.00128543 arcsec longitude**, **0.00000436 arcsec latitude**, **1.51950e-11 AU range**, below predeclared 1 arcsec and 1e-9 AU. Horizons query epochs are rounded to 9 JD decimals. No independent velocity validation is claimed here. This neither establishes physical orbital accuracy independently of DE441 nor validates ancient frame accuracy, local times, house angles, or the full application.

## Commands, evidence and gates

Windows 11; `cargo 1.99.1 (5f94df478 2026-08-27)`, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`.

```powershell
cargo fmt --all -- --check
cargo test --workspace --offline --quiet
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo build --release --offline -p virgostars-ephemeris-core --example audit_historical_ut1_provider
python scripts/audit_historical_ut1_provider.py <source.eph> <manifest.json> <pack-dir> <audit-exe> <new-output-dir>
```

Result: fmt and Clippy pass; **524 passed, 0 failed, 31 ignored** in workspace tests. Clippy initially flagged a large `const` array; changed to `static`, then all checks and the numerical audit were repeated. Compiler incremental-cache hard-link warnings on this filesystem resulted in copies, not test failures. The audit requires pyerfa and network access to Horizons, and creates a new output folder only.

Actual inputs: `D:\GitHub\virgo-stars\database\effemeridi\astrodienst\de441.eph`, `D:\VirgoStars-DE441-Mobile\packs\de441-mobile-manifest-v1.json`, `D:\VirgoStars-DE441-Mobile\packs`, and `target\release\examples\audit_historical_ut1_provider.exe` (SHA-256 `64d141c5e0e427547ce151a56ae0b2c58232769bbd400d6b244fdf107e641eef`). Evidence directory: `C:\Users\Windows11\.codex\visualizations\2026\11\18\01a1ec76-743f-79f2-a39d-b5623ef17cab\science-evidence-20261118\fairhead-time-run-02`; `report.json` SHA-256 `22e65b49d80e8fa70d6cadfdfd80b3852373af829ad59a351341b2fe22952eba`, `stdout.csv` SHA-256 `b9bc9743adca2854280bd97d26624a0f1475789eb25585e39753be6d90722688`. Raw JPL responses and all epoch residuals are in the local JSON.

Remaining: scientifically defensible historical Delta T and its uncertainty, civil clock/calendar/timezone provenance, independently validated long-term orientation and houses, any topocentric timing effects, and end-to-end application testing. The derivative of the rotating frame continues to use the existing per-day approximation; this audit is not an independent rate-validation campaign. No application, release or production activation was changed.
