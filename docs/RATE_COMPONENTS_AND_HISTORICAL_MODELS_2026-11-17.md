# Rate-component investigation and historical models — 2026-11-17

**Status: experimental investigation, not a release acceptance test.** This
continues [the earlier rate and historical-time comparison](PLANET_RATES_AND_HISTORICAL_TIME_2026-11-17.md).
No production calculation or default has changed.

The [subsequent solar-separation and historical-time investigation](SOLAR_RATE_OUTLIERS_AND_HISTORICAL_TIME_2026-11-17.md)
tests a further rate hypothesis and compares 39 historical epochs with Horizons.

## Horizons apparent ICRF rate convention

The earlier 610 cases (ten geocentric bodies, 1 July at astronomical years
0–3000 every 50 years) suggested that Horizons quantity 46 may be closer to a
projected velocity that does not include all derivatives of the corrections
applied to apparent position. This is a **hypothesis**, not a finding about
Horizons' internal implementation. The existing engine's rate agrees much more
closely with a centered numerical derivative of Horizons' *printed positions*
than with its printed rates; see the preceding report for those checks.

To challenge the hypothesis we fetched a separate grid, every ten years from
0 to 3000, using the same body/center/DE441/ICRF/airless and approximate TT/TDB
configuration as before. The 61 years divisible by 50 were used to select the
hypothesis and were excluded from the following **2,400 holdout cases** (240
epochs × ten bodies). This is a new set of epochs, not an independent dynamical
ephemeris: both sides use DE441. Horizons rates have finite printed precision.

The diagnostic projects the same apparent position against alternative rates.
The deliberately incomplete variants are artificial hybrids, **not physical
apparent states**, and must not be used as an ephemeris or a replacement for
the engine's derivative. RMS and maxima below are the magnitude of the two
component residual `(RA rate × cos(dec), Dec rate)` in arcseconds/hour:

| Diagnostic variant | All RMS | All max | Moon RMS | Jupiter barycenter RMS |
| --- | ---: | ---: | ---: | ---: |
| Full engine derivative | 0.040942 | 0.193257 | 0.125724 | 0.009991 |
| Omit Earth's acceleration in aberration derivative | 0.043695 | 0.207436 | 0.135740 | 0.000559 |
| Apparent position with untransformed velocity | 0.004369 | 0.023056 | 0.004980 | 0.000966 |
| Untransformed velocity, no emission-time rate | 0.000950 | 0.023053 | 0.000355 | 0.000558 |

Thus the last hypothesis reduces aggregate RMS by about 43× on the holdout,
but the maximum remains above the earlier 0.01″/hour investigative threshold.
The individual effects interact; this experiment does not isolate a unique
cause, establish what Horizons computes, or validate velocities in ecliptic of
date. Some large residuals remain to investigate, including behavior near the
Sun and reference precision. The small TT/TDB approximation and shared kernel
also limit interpretation. Follow-up should compare documented conventions and
targeted conjunction cases against independently generated references.

The new direct-rate grid exceeded the earlier component-wise 0.01″/hour
diagnostic limit in 1,190 of 3,010 cases. This failure remains explicit; the
hybrid comparison is not an acceptance criterion or a reason to change the
physical derivative.

## Historical-time model comparison

[Morrison et al.'s 2020 addendum, supplementary table S6](https://s3-eu-west-1.amazonaws.com/pstorage-rs-4828782598/26513566/rspa20200776_si_001.pdf)
publishes a more recent historical reconstruction through 2019. As a small
spot check, compare the table's **rounded** TT−UT1 values with the existing
experimental Espenak–Meeus polynomial at the same numeric astronomical year:

| Year | Addendum table, s | Existing polynomial, s | Polynomial − table, approximately s |
| ---: | ---: | ---: | ---: |
| 0 | 10440 | 10583.6 | +144 |
| 500 | 5590 | 5710.0 | +120 |
| 1000 | 1650 | 1574.2 | −76 |
| 1500 | 290 | 198.3 | −92 |

These are differences between reconstructions, not measured errors, sigma
estimates or independent validation; historical observations can overlap.
The cited table rounds these four values to ten seconds. Different underlying
lunar ephemeris/tidal assumptions may also matter and need separate analysis.
The addendum's historical spline does not cover year 3000; its future
uncertainty cannot be inferred from these samples. No external dataset or
coefficients were added to the repository, and the current time model has not
been replaced. Future work should establish provenance, uncertainty semantics,
ephemeris compatibility and the mapping from fractional years before adding a
selectable newer model. The earlier NASA uncertainty examples remain
illustrative, not bounds on this model comparison.

## Reproduction and provenance

Build the Rust example, then from `bindings/node` explicitly supply the classic
DE441 file and a previously acquired reference report. The report generator
makes live public requests to Horizons; the component audit itself is offline.
Both require a new output path. Input hashes bind the bytes, not the scientific
authenticity of the originating service. The report retains raw API responses
only in ignored local storage. `cargo fmt` and the other checks do not retrieve
astronomical data.

```text
cargo build --offline --locked --example diagnose_icrf_rate_components
node scripts/compare-horizons-icrf-rates.cjs <de441> <sha256> <new-reference> 0-3000-step-10
node scripts/audit-horizons-rate-components.cjs <de441> <sha256> <reference> <reference-sha256> <diagnostic-executable> <new-report>
```

The first command runs from the repository root, the Node commands from
`bindings/node`. The Horizons script exits nonzero when its investigative
component threshold is exceeded while still writing its complete report.
The audit validates unique body/epoch pairs, both input hashes and its Rust
executable hash; it does not independently authenticate the source report.
Reference report SHA-256:
`eede3c5ba9802e37c022e6ebb783af8a15471c0ad8232ed2169ccbfcf953e69c`;
component report SHA-256:
`c77c83d7f73507b0b3d81585425aedafba0c2f85a0f3aa9e7bca38127fd420c4`.
The component report also records the executable and DE441 hashes. Local
reports and the external kernel are not committed.
