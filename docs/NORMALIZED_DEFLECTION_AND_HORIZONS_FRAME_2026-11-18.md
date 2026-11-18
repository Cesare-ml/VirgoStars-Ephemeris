# Normalized solar deflection and Horizons frame isolation

**Status: offline experiment.** No production function, binding, default,
stellar catalog, or server has been changed. This continues the
[solar-deflection candidate and ancient-reference audit](SOLAR_DEFLECTION_RATE_CANDIDATE_2026-11-18.md).

Continuation: [equatorial comparison and individual floor stencils](HORIZONS_EQUATOR_AND_DEFLECTION_STENCILS_2026-11-18.md).

## Normalized correction and real DE441 cases

The production deflected position has the form `R normalize(p + δ)`, with
range `R`, unit light-time direction `p` and solar deflection `δ`. The
experimental example now differentiates this normalized expression, including
the range-rate term for the Cartesian velocity. A synthetic test checks the
Cartesian derivative against independently sampled positions with changing
range, in addition to separate direction and deflection tests. This is a
mathematical consistency check, not an independent physical validation.

At the 121 previously sampled cases (105 distinct body/epoch pairs) and their
242 offset cases (210 distinct), a fixed-ICRF angular-rate correction is
compared with a fourth-order derivative of the *same normalized correction*
at a 1/2048-day half-step. A 1/4096-day reference measures step sensitivity.
The candidate differentiates the existing deflection expression analytically;
the existing pipeline uses its current numerical deflection derivative. Both
retain the same deflection model and light-time inputs. Some samples represent
occulted geometry, and these are not estimates of visible-sky accuracy.

| Grid | Existing maximum residual | Candidate maximum residual | Reference disagreement maximum | Candidate smaller |
| --- | ---: | ---: | ---: | ---: |
| Initial 121 rows | 0.001527594″/day | 0.000001559″/day | 0.000000949″/day | 74/121 |
| Offset 242 rows | 0.038670778″/day | 0.000001565″/day | 0.000001564″/day | 140/242 |

Maxima can occur at different samples. Many residuals are comparable with
reference sensitivity, so the counts are not a reliable ranking of every
case. These checks cover the deflection correction's normalized direction,
not the entire apparent ecliptic pipeline or a continuous time interval.

## Crossing the model's denominator floor

The existing model uses `max(q·(q+e), 1e-6)`. It is continuous but has a
derivative kink at the floor. A scan of 257 binary-spaced times near one
previously identified Uranus case returns 257 candidates and finds **32**
where at least one diagnostic stencil straddles that floor. These are
near 0.07–0.08° from the Sun and are occulted-case numerical diagnostics.
Near the kink the two reference step sizes differ by up to **13.78″/day**;
there is no single smooth derivative at the boundary, and a centered
finite-difference result there must not be treated as an accuracy oracle.
The candidate rejects epochs within its specified boundary tolerance; none
of the 257 sample centers fell inside it. Choosing an API policy for the kink
and validating any handling of it remain open before production integration.

## Horizons: fixed ICRF versus ecliptic of date

For the same seven TT epochs and the same geocentric Sun used in the earlier
quantity-31 comparison, a new request obtains Horizons quantity 45: airless
apparent ICRF direction. Horizons labels both Sun and Earth as DE441. The
local TDB input is obtained from TT with a one-term approximation. At
astronomical year −9998, local and Horizons ICRF directions differ by about
**0.00000133″** in total angular separation, while the earlier local-minus-
Horizons ecliptic longitudes differ by **−210.94″**. Across all seven epochs,
the largest ICRF separation is **0.00000274″**.

This strongly narrows the large Sun discrepancy to the transformation and
conventions used for coordinates of date, rather than the underlying Sun
direction at those sampled instants. It does not isolate precession from
nutation or the ecliptic-plane convention, prove an individual model correct,
or establish a full-range accuracy bound. Horizons output rounding and the
TT/TDB approximation also limit interpretation at microarcsecond scale.
Horizons quantity 31 uses IAU76/80 and long-term Owen orientation; the local
paths use Vondrák-derived axes and extrapolated nutation. See the
[Horizons reference manual](https://ssd.jpl.nasa.gov/horizons/manual.html).

## Reproduce

The same separately supplied DE441 and the exact local reports from the
previous audit are required; the scripts verify SHA-256 of inputs before and
after, require fresh output paths and retain no data in Git. The previous
baseline report was created before its script added the optional `grid` field;
its archived hash below identifies that original report, not a byte-for-byte
promise for a fresh replay with the newer script.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
cargo test --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
node bindings/node/scripts/audit-normalized-deflection-rate.cjs <de441> <kernel-sha256> <prior-report> <prior-sha256> <example-executable> <new-report>
node bindings/node/scripts/audit-deflection-floor-crossing.cjs <de441> <kernel-sha256> <example-executable> <new-report>
node bindings/node/scripts/compare-horizons-frame-isolation.cjs <de441> <kernel-sha256> <ecliptic-report> <report-sha256> <new-report>
```

Local report SHA-256s:

- Normalized baseline: `be9c7833e932d211909a3f24a64a0bf7581087e777ab066e208be7e71835f437`.
- Normalized offsets: `75185d0458f5107e38cfe24299a0b9c4f424197834d4fc73f5054a0fb6568205`.
- Floor scan: `eb2d1abac677e2609c9d00873396a65f86af08390097e4c8a852baebc4831270`.
- Horizons ICRF isolation: `6c5386dba265f236406482300e514f547a63d21038f3991320900432f96919ed`.
