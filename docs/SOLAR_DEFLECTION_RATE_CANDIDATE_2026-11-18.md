# Exploratory solar-deflection rate and ancient reference checks

**Status: research diagnostic only.** No default, production calculation,
public API, historical-time model, or stellar data is changed. This continues
the [ancient obliquity and ΔT audit](ANCIENT_OBLIQUITY_AND_DELTAT_2026-11-17.md).

Continuation: [normalized deflection, floor crossing and Horizons ICRF](NORMALIZED_DEFLECTION_AND_HORIZONS_FRAME_2026-11-18.md).

## Candidate rate derivative

The diagnostic differentiates the existing solar-deflection expression
analytically, including derivatives of observer-to-target, Sun-to-target and
Sun-to-observer unit vectors, distance, and the retarded emission time. It
keeps the existing denominator floor; at its kink the derivative is not
defined and the example declines to return a candidate. If samples straddle
the floor they are flagged. The candidate lives only in an offline example;
it has not been integrated into the apparent-coordinate pipeline.

Synthetic tests cover independently changing vectors and Sun distance, plus
source-at-infinity geometries above and below the floor and the kink itself.
They test the derivative of the implemented mathematical model, not whether
that model is physically adequate for an occulted or near-limb ray.

The DE441 diagnostic compares the **ICRF vector norm** of the deflection-only
rate difference, in arcseconds/TDB day, against a fourth-order numerical
derivative of the *same model*, using 1/2048-day half-steps. A second
1/4096-day derivative measures reference sensitivity. The existing two-point
method uses a 1/256-day half-step. It examines Mercury, Venus and Mars–Pluto
system barycenters; epochs near years −12999, −8750 and 2000 are chosen by
daily geometric sampling of two years at each era, targeting elongations near
0°, 1°, 5°, 30° and 90°, plus the earlier Uranus outlier. Those grids are
not exhaustive and include duplicates and physically occulted geometries.

| Grid | Rows (distinct body/epoch) | Existing maximum | Candidate maximum | Reference-step disagreement maximum | Candidate smaller |
| --- | ---: | ---: | ---: | ---: | ---: |
| Daily-selected + Uranus | 121 (105) | 0.001528″/day | 0.000001566″/day | 0.000000932″/day | 92/121 |
| ±0.125 day offsets | 242 (210) | 0.038671″/day | 0.000001567″/day | 0.000001624″/day | 179/242 |

No sampled finite-difference stencil crossed the floor. The larger offset
grid maximum occurs only 0.109° from the Sun and can be occulted; it is not
a realistic observational accuracy claim. Differences near the reference's
own sensitivity cannot rank methods reliably. The maximum figures occur at
different cases, and a smaller sampled residual is not a universal error
bound. No timing, floor-crossing dynamics, higher-order gravity, continuous
coverage or independent physical reference has been established. Further
work should explicitly address guard crossings and the normalized-vector
derivative before integrating or changing any public velocity.

## JPL Horizons orientation comparison

Seven fixed-TT Sun observations from 9999 BCE to 2000 CE were requested from
[JPL Horizons quantity 31](https://ssd.jpl.nasa.gov/horizons/manual.html),
geocentric, airless, with apparent light-time and corrections. Horizons reports
DE441 for both Earth and Sun. Its of-date IAU76/80 convention uses
[Owen long-term precession outside 1799–2202](https://ssd.jpl.nasa.gov/horizons/manual.html#precession-model),
while the tested alternatives use long-term Vondrák pole and mixed frames with
extrapolated nutation; the two conventions are not identical. The local input
is approximate TDB converted from TT using one periodic term. The comparison is
not an independent planetary-orbit solution or a way to assign physical blame.

For astronomical year −9998, local apparent longitude minus Horizons is
approximately −210.94″ for both alternatives; latitude differences are
−3.74″ for the pole frame and −34.69″ for the mixed frame. At year 2000 the
longitude difference is +0.049″ for each. Seven samples are not a full-range
accuracy assessment. This reference highlights an unresolved long-range
orientation-convention difference despite close Swiss agreement.

## Fixed-UT1 historical time sensitivity

On 26 sampled UT1 instants from 13000 BCE through 720 BCE, the 2006 NASA and
2016 Stephenson–Morrison–Hohenkerk parabolas differ by 105–11161 seconds;
the direct mixed-frame Moon longitudes computed with those choices differ by
1.05–109.83 arcminutes. Both parabolas are extrapolated beyond their documented
regimes for the ancient samples, and the later parabola is not its published
observational spline. These are scenario differences, not truth or uncertainty
bounds. The one-hour uncertainty parameter in the diagnostic is arbitrary and
is not used to infer an error bar.

## Reproduction and evidence

Supply a separately obtained classic DE441 file and its SHA-256; reports
remain local and ignored. The audit scripts check the kernel hash before and
after. Horizons requires network access; other commands are offline.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
cargo test --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
node bindings/node/scripts/audit-solar-deflection-candidates.cjs <de441> <sha256> <example-executable> <new-report> baseline
node bindings/node/scripts/audit-solar-deflection-candidates.cjs <de441> <sha256> <example-executable> <new-report> offset
node bindings/node/scripts/compare-horizons-ancient-ecliptic.cjs <de441> <sha256> <new-report>
node bindings/node/scripts/audit-ancient-delta-models.cjs <de441> <sha256> <swisseph> <new-report> dense
```

Locally generated report SHA-256s, in command order:

- Baseline: `5c46c20b8436244142f144111e31ffa51697bc54d3a2b0fa2622326627850714`.
- Offset: `ebe492d0521ac8980779aea287b2ddb2bf6586a7828f1d183e67192c3c2c696f`.
- Horizons ecliptic: `ea9a020d411f6ef1ea48068804cf5efbeb959aa98dcd089e7e9335915fad0ccc`.
- Dense fixed-UT1: `f8f0dcdd3bc8966fd259d07867a424df2ab73a3255a4a919e4a26bd29cc2b6e5`.
