# Solar-rate outliers and historical-time comparison — 2026-11-17

**Status: experimental diagnostics, not production or release validation.**
This follows [the rate-component investigation](RATE_COMPONENTS_AND_HISTORICAL_MODELS_2026-11-17.md).
No production calculation, public API or default time model changed.

A [subsequent seasonal velocity comparison](PLANETARY_SPEED_VALIDATION_SEASONAL_2026-11-17.md)
checks geometric vectors directly and apparent ecliptic rates against Swiss.

## Association with solar separation

Reanalyzing the existing 2,400 previously held-out Horizons cases after
selecting the first hybrid, we grouped the 2,160 non-Sun targets by their
angular separation from the Sun, using both Horizons apparent ICRF directions
at the same TT epoch. Bins are half-open in degrees except the final endpoint.
The residual is the magnitude of the ICRF RA×cos(dec) and Dec rate-component
differences, in arcseconds/hour. The original hybrid combines apparent position
with velocity lacking aberration and emission-time derivatives. The new
hybrid additionally omits the solar-deflection contribution to velocity.
Both are deliberately inconsistent states, **not physical apparent rates**.

| Solar separation | Cases | Original hybrid RMS | Original max | New hybrid RMS | New max |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0–5° | 95 | 0.004734 | 0.023053 | 0.0000914 | 0.000659 |
| 5–10° | 106 | 0.000215 | 0.000604 | 0.000106 | 0.000604 |
| 10–30° | 486 | 0.000105 | 0.000616 | 0.0000984 | 0.000616 |
| 30–180° | 1473 | 0.000129 | 0.000674 | 0.000129 | 0.000674 |

The new hybrid was selected **after examining these cases**; this table is not
an independent validation of that selection. It suggests that the original
hybrid's largest residuals track a deflection-rate contribution, but does not
prove Horizons' internal implementation or identify every difference. Both
computations share DE441, and printed JPL rates have finite precision. The
unchanged full engine rate is the derivative of its apparent position; a
hybrid's better match is not justification to remove the physical derivative.

For an additional targeted check, we queried Horizons at ten **new** TT epochs,
one day before and after five previously selected large-residual cases (Venus,
Saturn, Uranus and Neptune). These are adjacent epochs selected based on the
earlier data, not a random or independent ephemeris sample. The original hybrid
had RMS 0.486584″/hour and maximum 1.534564″/hour; the new hybrid had RMS
0.0000332″/hour and maximum 0.0000881″/hour. The maximum original difference
was Saturn barycenter, year 120, one day later: our apparent direction was
only about **0.098° from the Sun's center**, inside the solar disk. Such a
case is occulted, not an observable sky position, and the engine's documented
solar-deflection model is not intended for rays passing within a few solar
radii. The adjacent Uranus year 610 case is about 0.393° from the Sun, also
close to the model's stated geometric limits. These probes test conventions;
they cannot establish physical rate accuracy there. For a usable release
criterion, choose supported solar separations and independently compare
physical derivatives of apparent positions with higher-precision references.

## Time reconstruction returned by Horizons

Horizons [documents](https://ssd.jpl.nasa.gov/horizons/manual.html) that its
pre-1962 quantity 30 is approximately TT−UT1 (it does not preserve the tiny
TDB/TT distinction for these historical epochs), with historical models based
on work including Stephenson et al. 2016 and the 2020 addendum. Its precise
internal interpolation and model selection are not asserted here.

We requested 39 dates: 1 July 12:00 TT, astronomical years 0–1900 every 50
years. For the existing experimental time model, decimal year is derived using
the proleptic Gregorian fraction of the same TT Julian date; Horizons displays
a mixed calendar but the query identifies the instant by JD. Using TT rather
than UT1 for the year fraction is a small approximation compared with the
model differences. The model uses its existing 1900–1972 table branch and is
otherwise the experimental Espenak–Meeus estimate. Values in seconds:

| Year | Existing estimate | Horizons historical reconstruction | Difference |
| ---: | ---: | ---: | ---: |
| 0 | 10578.54 | 10476.28 | +102.27 |
| 500 | 5705.11 | 5604.29 | +100.83 |
| 1000 | 1571.44 | 1657.60 | −86.17 |
| 1250 | 601.51 | 795.41 | −193.90 |
| 1500 | 197.80 | 293.62 | −95.81 |
| 1900 | −2.06 | −1.35 | −0.71 |

Across these 39 samples, maximum absolute difference was **193.90 s** (1250)
and RMS difference was **117.52 s**. These are model differences, not measured
errors or probability bounds; underlying observations may overlap. A modern
reconstruction is a useful candidate but cannot be declared accurate merely
because it is newer. The numerical results do not extend to year 3000. Before
an optional new implementation, resolve the exact dataset/rights, model epoch
and lunar-ephemeris assumptions, source uncertainty and interpolation, and
compatibility with existing explicit UT1 inputs. Avoid replacing current
behavior or implying that UTC existed at historical epochs.

## Reproduction and evidence

With the caller's classic DE441 and the original `horizons-rates-step10.json`,
build the diagnostic Rust example from the repository root, then run from
`bindings/node`:

```text
cargo build --offline --locked --example diagnose_icrf_rate_components
node scripts/audit-horizons-rate-components.cjs <de441> <sha256> <reference> <reference-sha256> <diagnostic-executable> <new-component-report>
node scripts/analyze-rate-solar-separation.cjs <reference> <reference-sha256> <component-report> <component-sha256> <new-report>
node scripts/probe-horizons-solar-neighborhood.cjs <de441> <sha256> <diagnostic-executable> <new-report>
node scripts/compare-horizons-historical-deltat.cjs <new-report>
```

Only the final two commands make explicit live Horizons requests; external
responses and DE441 remain ignored local data. Component report SHA-256
`bb5f78136919bbbde84a4af92f79ae966cd69f60123da237231de0495aaf2378`;
historical report SHA-256
`6ede88b20f3a2a8052397c77dd895fb760d6cd7c9d89f5166f2156875d050a67`.
The separation report SHA-256 is
`b49506c2ffe95e40397e330b392dc72feec7c757d2ce8c1c10494d464d010dde`;
the ten-epoch neighborhood report SHA-256 is
`348e8da82f25dd49074966b4b0e21dd2e0f9583b08c9ebec17e245dbfd143b64`.
Hashes do not authenticate the external publisher. See the previous report for
the original reference and kernel hashes, frame, corrections and limitations.
