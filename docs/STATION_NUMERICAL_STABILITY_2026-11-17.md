# Station numerical stability and Horizons directions — 2026-11-17

**Status: exploratory, sampled diagnostics; no production fix or event-time
accuracy certification.** This extends [the 944-case station matrix](SWISS_STATION_MATRIX_2026-11-17.md).
The canonical engine, its APIs and production activation are unchanged.

A [subsequent engine-only convergence check](ENGINE_STATION_CONVERGENCE_2026-11-17.md)
compares the returned rates with derivatives of the engine's own longitudes.

## Swiss rate behavior in short windows

We selected 48 cases from the existing matrix: both directions for Mercury
through Pluto at the 50, 1500 and 2950 year seeds. Three previously observed
bracket-sensitive cases were added, for **51 total**; this selected group is
not statistically representative. Around each previously bracketed Swiss
published-speed zero, we sampled 121 points at 0.5-second spacing over ±30
seconds, using Swiss 2.09.03/JPL mode with all requested flags checked. We
removed a fitted straight line from each rate series; the residual RMS
describes short-window departures from that fitted line, **not** a statistical
uncertainty or a physical error bound.

| Swiss mode | Median per-case detrended RMS, ″/day | 95th percentile, ″/day |
| --- | ---: | ---: |
| Default apparent | 2.19e-5 | 3.33e-5 |
| Gravitational deflection disabled | 2.77e-7 | 5.53e-7 |
| Annual aberration disabled | 2.19e-5 | 3.33e-5 |
| Nutation disabled | 2.19e-5 | 3.33e-5 |
| True position | 2.48e-7 | 5.47e-7 |

These are observations about **one installed Swiss version and selected
windows**. They suggest that the rapid fluctuations are associated with the
deflection option; the true-position mode changes multiple corrections. This
does not establish the internal implementation or justify turning off a
physical correction. It is distinct from the earlier finding that the broader
offset between Swiss's published-speed and position-derivative stations was
associated with the nutation option.

At 0.5-second sampling, 12 of these 51 default Swiss series changed sign more
than once within the one-minute window; the others changed sign once. Such a
sampled sequence can contain several zero crossings or rapid fluctuations and
does not by itself establish a unique physical station. Repeating the exact
same epoch 20 times, interleaved with other epochs and bodies, returned one
identical speed value per case: this specific check found no call-order effect.
It does not establish determinism for every input or environment.

We reran bisection using three differently positioned, non-aligned one-day
brackets. The spread of returned center times across those brackets was at
most **2.51 s** for Swiss's published rate, versus **0.57 s** for Swiss
position derivatives and for the engine on this selected set. A straight-line
fit to the sampled Swiss rate gave a zero up to **3.14 s** away from the
original Swiss bisection center. The fit is a diagnostic smoothing experiment,
not an endorsed station algorithm or confidence interval. Other brackets and
dates previously produced larger variation, and a narrow numerical bracket is
not sufficient evidence of a unique or accurate event time.

The Swiss position-derivative comparison itself has numerical sensitivity:
across the 51 windows its detrended RMS ranged substantially by body, with a
95th percentile of **5.04e-5″/day**. The 0.02-day half-step, printed/returned
precision, sampled time grid and frame differences all affect that value.
It must not be treated as an exact oracle near zero.

## Horizons position derivatives near eight stations

We requested apparent airless ICRF right ascension/declination from JPL
Horizons around eight selected retrograde stations near the 1500 seed, one
per Mercury through Pluto system barycenter. Horizons output was checked for
body, Earth center, DE441, ICRF and TT epochs. At each sampling epoch, the
printed direction was projected to true ecliptic of date using **the same
experimental VirgoStars orientation**. Its ecliptic longitude derivative was
estimated with fourth-order differences and compared with the engine's own
apparent longitude rate at the station. The projection was also checked
against the engine's projection of its own ICRF vectors.

The maximum absolute component residual over the eight cases was:

| Half-step (TDB days) | Maximum rate residual, ″/day |
| ---: | ---: |
| 0.01 | 0.0001751 |
| 0.02 | 0.0000592 |
| 0.04 | 0.0000467 |

The result depends materially on the step and finite precision of the printed
directions, especially because the rate is near zero. The test supports the
sampled rate comparison but **does not determine a sub-second station time**.
Horizons and the engine share DE441, and the projection shares the engine's
orientation, so it is not independent validation of the orbit, precession or
nutation model. TDB/TT conversion is approximate; UT1/ΔT and historical civil
time remain separate uncertainties. The three samples are not a continuous
accuracy bound or release threshold.

## Reproduction and evidence

After building the native Node binding, build the projection example from the
repository root, then run the Node commands from `bindings/node`. Provide the
caller-owned classic DE441, its hash, a caller-installed Swiss module, the
previous local station matrix and its hash, plus new output paths. Only the
Horizons command makes explicit live requests; repeat it with half-steps 0.01,
0.02 and 0.04. The projection executable's hash and raw JPL responses are
retained in the ignored local reports.

```text
cargo build --offline --locked --example diagnose_icrf_ecliptic_projection
node scripts/audit-swiss-station-smoothness.cjs <de441> <sha256> <swisseph> <matrix> <matrix-sha256> <new-report>
node scripts/compare-horizons-near-stations.cjs <de441> <sha256> <matrix> <matrix-sha256> <projection-executable> <new-report> 0.02
```

Local SHA-256: smoothness report
`9f2437d32488607eae83625f4cdec47fcf7184831842ea064583d3230366cda4`;
Horizons `h=0.01`
`da368effcba805bdce951646ec42658715966541b229592c744a5d0c8bcfd859`;
`h=0.02`
`9c9acebac2b74dc3f1f7f864b1192752fd4b3da2eac61e22f5a5be32e2548784`;
`h=0.04`
`01b1e7092f4548b1000fab9c44c6298399d6101d71cb2594e17b714ed10631d2`.
Hashes identify local bytes, not source authenticity. No external kernel or
raw reference dataset is committed.
