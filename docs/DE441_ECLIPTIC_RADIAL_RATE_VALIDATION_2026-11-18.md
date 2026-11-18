# Independent ecliptic and radial rate comparisons — 2026-11-18

**Result: the sampled standard Rust apparent ecliptic and radial rates agree with derivatives of JPL Horizons positions and ranges within the predeclared diagnostic limits. Direct Horizons Q20 radial rates do not consistently agree with the derivative of Horizons' own ranges.** No app engine is activated. This is an implementation comparison using the same DE441 orbit, not an independent estimate of the orbit's physical accuracy. The sample, separate comparisons and thresholds were fixed in [the plan](DE441_ECLIPTIC_RADIAL_RATE_PLAN_2026-11-18.md) before requesting the reference data.

Follow-up: [station and boundary tests](STATIONS_BOUNDARY_RATE_VALIDATION_2026-11-18.md) found and corrected reference-stencil timing in the audit scripts. An offline TT-weighted replay of the 90 cases below preserved the reported maxima and failures; the new boundary samples were sensitive to this correction.

## Reference and method

Use 90 geocentric target/date cases: Sun 10, Moon 301, Mercury 199, Venus 299 and Mars–Pluto **system barycenters** 4–9, each at July 1 12:00 TDB in 1901, 1925, 1950, 1975, 2000, 2025, 2050, 2075 and 2099. The standard Rust `geocentric_apparent_six_fields` uses the local classic DE441, light time, solar deflection, annual aberration, and IAU 2006/2000B true ecliptic of date. Its input and output speeds are per TDB day.

Horizons provides airless apparent ICRF Q45 directions and light-time-adjusted Q20 range/deldot for Earth center 399, with DE441 as source. Its request times are JD TT, produced by the Rust core's approximate TDB→TT conversion. Each Q45 direction is independently rotated at **its own TT epoch** using ERFA `nut00b`, `obl06`, `numat` and `pmat06` to the same model's true ecliptic of date. Thus finite differences include the changing axes. Horizons Q31 uses IAU 1976/1980 ecliptic of date and is not substituted for this common-model reference. Longitude is unwrapped around each center; a four-point cubic derivative uses the actual representable TDB JD offsets. Reference positions are sampled at ±0.02 and ±0.04 day, and independently at ±0.04 and ±0.08 day. Horizons output precision, approximate time-scale conversion, finite difference and shared model/ephemeris limit the comparison.

## Results against position and range derivatives

Both stencils for each of the 90 cases are counted: **180 comparisons**. Limits below were set before fetching and are exploratory, not release-accuracy requirements.

| Rust field versus JPL/ERFA derivative | Limit | Maximum absolute residual | Above limit |
| --- | ---: | ---: | ---: |
| Ecliptic longitude rate | 0.05″/TDB day | 0.0002701″/day, Moon 2099, h=0.02 | 0/180 |
| Ecliptic latitude rate | 0.05″/TDB day | 0.0001072″/day, Neptune barycenter 2025, h=0.02 | 0/180 |
| Radial distance rate | 5e−10 AU/TDB day | 9.149e−12 AU/day, Uranus barycenter 2099, h=0.02 | 0/180 |

There were no longitude-rate sign mismatches across the 90 paired cases. The closest sampled planetary rate to zero was Uranus barycenter in 1925, about −0.0005543°/day; this does not validate exact station times. The maximum difference between the two stencils was 0.0001550″/day for longitude, 0.0001301″/day for latitude and 5.04e−12 AU/day for range. Those differences are sensitivity indicators, not error bounds. The closest non-Sun sample to the Sun was Mars barycenter in 2000 at 0.8724° elongation; it is a numerical stress case, and the result does not establish physical accuracy or visibility close to the Sun.

## Direct Q20 range rate — comparison fails its separate limit

Horizons Q20's printed `deldot` was independently compared with the numerical derivative of its own `delta` range. With the separately predeclared 0.001 km/s diagnostic threshold, **22 of 180 comparisons** exceed it: both stencils at **11/90** target/date cases, seven Mercury and four Venus. The largest is Mercury in 1975: Q20 reports **33.7223560 km/s**, while the derivative of its printed range is **33.7173940 km/s**, a difference of **+0.0049620 km/s**. The two stencils agree on the discrepancy; the successful Rust-versus-range-derivative checks above do not turn the direct Q20 comparison into a pass.

After observing that discrepancy, a separately planned diagnostic used the existing Rust light-time solver on Sun, Moon, Mercury, Venus and Jupiter barycenter at all nine dates (45 cases). Projecting its correctly differentiated light-time velocity on the line of sight agrees with the JPL range derivative within 0.00001 km/s in **45/45** cases (maximum 1.17e−8 km/s). Projecting instead the deliberately incomplete relative velocity `V_target(emission) − V_Earth(reception)` agrees with the printed Q20 rate within the same threshold in **45/45** cases (maximum 4.95e−8 km/s). For the Mercury 1975 example, these diagnostic projections are **33.71739403** and **33.72235603 km/s** respectively.

The diagnostic strongly supports a different treatment of the emission-time derivative as the explanation for these sampled Q20 differences. It does **not** establish JPL's internal algorithm or imply that the incomplete velocity is a physical range derivative. The follow-up reused observed cases and is not a holdout. A question about Q20 has been added to the prepared, unsent JPL inquiry.

## Reproduction and local evidence

The caller-owned classic DE441 is checked before and after with SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`. The reference uses the same orbit, not an independent dynamical solution. With Python 3.12, pyerfa 2.0.1.5, NumPy 2.5.5 and network access, from the repository root:

```text
cargo build --release --offline -p virgostars-ephemeris-core --example apparent_six_fields --example audit_retarded_velocity
python scripts/audit_horizons_ecliptic_radial_rates.py <kernel> <sha256> <apparent-executable> <new-report.json>
python scripts/audit_q20_radial_convention.py <kernel> <sha256> <retarded-executable> <new-report.json> <report-sha256> <new-q20-report.json>
python -m unittest discover -s scripts -p test_audit_horizons_ecliptic_radial_rates.py -v
```

The first Python command writes the complete report and exits **1** because the Q20 direct-rate limit fails; it is not a network or compile failure. It preserves the 10 full query URLs, response metadata and raw tables in the owner's local report, not the repository. The follow-up exits 0 for this run. Local evidence SHA-256: `ecliptic-radial-horizons-erfa.json` `236aebdc1cdda25555d8682de290251713bf9efe34d7cd9fe28aec7159c7de1a`; `q20-radial-convention.json` `0b5858ce12f5b87b9381d407df8e7e70f163a940eabb106d336da8eb163b4148`. No kernel or raw JPL response is distributed.

On Windows, the four-point stencil's independent cubic/uneven-offset unit test passed 1/1. `cargo test --workspace --offline` passed 508 tests with 31 ignored; the four ignored real-DE441 apparent tests passed separately. `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, and the release builds of both diagnostic examples passed.

These samples do not validate every date, exact station times, planetary centers beyond the inner planets, topocentric or historical civil-time conversions, model behavior near solar occultation, or independent physical ephemeris uncertainty. The moving-frame convention is compared with the same IAU 2006/2000B model and does not by itself establish physical Earth-orientation accuracy.
