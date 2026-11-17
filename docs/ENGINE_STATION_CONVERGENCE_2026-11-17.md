# Engine station derivative convergence — 2026-11-17

**Status: experimental self-consistency evidence, not physical accuracy or
release certification.** This follows [the Swiss and Horizons numerical
investigation](STATION_NUMERICAL_STABILITY_2026-11-17.md). No production
calculation, event API or consumer configuration changed.

## What was tested

We selected **80 events** from the previously retained station matrix: both
directions for Mercury, Venus and Mars–Pluto system barycenters at year seeds
100, 500, 1500, 2500 and 2900. The matrix supplies approximate event locations;
this diagnostic subsequently calls only the VirgoStars engine and the caller's
classic DE441. Events are geocentric, apparent true-ecliptic-of-date longitude
stations on the TDB scale, using the existing experimental long-range path.

For each event, we separately bracketed the zero of the returned longitude
rate and the zero of a fourth-order finite derivative of the engine's **own
longitudes**. Position derivative half-steps were 0.005, 0.01, 0.02 and 0.04
TDB day, using actual floating-point JD spans and wrapped longitude. Each
root was searched from three non-aligned initial brackets, with a nominal
final bracket width below 1e-7 day. The matrix and kernel hashes were verified
before and after. No external ephemeris or reference data is committed.

## Sampled results

All 80 selected events were bracketed for every half-step and initial
interval. Across the four half-steps, the maximum absolute difference between
the first position-derivative root and the first returned-rate root was about
**0.025 s** (Pluto system barycenter, direct motion, near the 2900 seed). The
95th percentile was approximately **0.020 s**. The three bracket centers for
the returned-rate root spanned at most **0.0046 s**; for position-derivative
roots they spanned at most **0.0071 s** on this grid. These values describe
the chosen numerical methods and samples, not an uncertainty bound.

On a ±2-second grid sampled every 0.1 second around the selected returned-rate
roots, both returned rates and the 0.02-day position derivatives showed one
sampled sign change in all 80 cases. This does not exclude additional changes
between grid points or on other dates.

The result is **not independent validation**: positions and rates share the
same code, DE441, frame, time conversion and physical model. A close match can
coexist with a shared error. It does not validate all dates or positions, the
extrapolated precession/nutation, actual orbit accuracy, JPL/Swiss station
parity or historical civil times. In particular, a 25-millisecond sampled
agreement must not be marketed as 25-millisecond physical station accuracy.
Independent Horizons checks remain limited by the printed direction precision
and shared orbital/orientation models, as documented in the preceding report.

## Regression and reproduction

An opt-in Node regression also uses `DE441_PATH` to locate both directions for
each of the eight bodies near a year-2000 TDB seed, then checks returned rates
against finite derivatives of the engine's own longitudes. Its `1e-5″/day`
limit is a generous local regression guard, not a release or physical error
budget. It skips if the caller does not provide a kernel.

From `bindings/node`, after building the native binding, supply the local
classic DE441, the prior ignored station-matrix report, their SHA-256 hashes,
and a new output path:

```text
node scripts/audit-engine-station-convergence.cjs <de441> <kernel-sha256> <matrix> <matrix-sha256> <new-report>
node --test test/engine-station-convergence.test.mjs
```

Set `DE441_PATH` explicitly before the test to execute the real-kernel case.
The local convergence report SHA-256 is
`d54951e7f8d88c6414045e9e55e3085d2573dcac4cdc146c9f07a2533d99cb53`.
The source kernel and matrix hashes are recorded in the report and preceding
documentation. Hashes identify local bytes, not source authenticity.
