# Planetary rates and historical time — 2026-11-17

**Status: experimental, local diagnostics; rate and historical-time release
criteria remain open.** This follows
[the 0–3000 positional comparison](PLANET_VALIDATION_0_3000_2026-11-17.md).
No production server or desktop provider is activated.

A [subsequent holdout investigation](RATE_COMPONENTS_AND_HISTORICAL_MODELS_2026-11-17.md)
tests rate-convention hypotheses on additional epochs and compares selected
historical time-model values with a later published reconstruction.

## Apparent ICRF rates versus Horizons

At 61 dates (1 July, astronomical years 0–3000 in steps of 50) and ten
geocentric targets, the engine's experimental apparent ICRF Cartesian
position and velocity were converted to `dRA/dt × cos(dec)` and `d(dec)/dt`,
in arcseconds/hour. These were compared with [JPL Horizons observer quantity
46](https://ssd.jpl.nasa.gov/horizons/manual.html), after matching the target,
Earth center, DE441 source, ICRF, airless corrections, TT epochs and columns.
Engine input is TDB; the reference uses approximately corresponding TT. Solar
light-time, deflection and annual aberration are included. The six targets
Mars–Pluto are system barycenters. Horizons prints a target-dependent number
of decimal places, so its reported rate precision is finite.

The predeclared **investigative** limit of 0.01″/hour was exceeded in **243 of
610 cases**. The largest component differences were 0.18653″/hour in the
RA×cos(dec) component (Moon, year 100) and 0.07945″/hour in declination
(Moon, year 1800). For Mercury, the maximum RA×cos(dec) difference was
0.03163″/hour. This is not a successful direct rate parity test.

To investigate, the Horizons positions at ±0.001 TDB day were differenced,
with RA wrap and cos(dec) handled explicitly. Across the same 610 cases, the
largest difference between the engine's rate and that finite difference of
Horizons positions was **0.001664″/hour** in RA×cos(dec) and
**0.000643″/hour** in declination, both Moon at year 500. All 610 fell under
the predeclared **0.005″/hour diagnostic limit**. The engine's own rate versus
its own position finite difference differed by at most 0.000017″/hour in
RA×cos(dec) and 0.000013″/hour in declination. Printed-angle and time
quantization, the finite-difference step and TT/TDB approximation affect the
comparison; the bound is neither a continuous guarantee nor independent
orbital truth because both implementations use DE441.

In five selected cases, Horizons' *reported* rate was also compared to its
own position finite difference. For Moon in year 100, the reported RA×cos(dec)
rate was 2049.165″/hour, while the finite difference at ±0.001 day was
2049.352″/hour; the engine gave 2049.352″/hour. For Jupiter's barycenter in
year 2000 the analogous values were 30.44644, 30.45517 and 30.45513″/hour.
At ±0.005 day these qualitative discrepancies persisted in the selected
cases. This isolates a **definition or implementation difference still to be
resolved**, not proof that either provider's rate is wrong. The 610-case
finite-difference result compares against Horizons positions, not its reported
rate; ecliptic-of-date rates and acceptance thresholds require separate review.

For a separate Swiss ecliptic-of-date comparison at 301 July epochs, changing
the numerical half-step from 0.001 to 0.005 or 0.01 day changed the maximum
lunar longitude-rate residual from 0.3052″/day to 0.05393 and 0.03024″/day.
At 0.01 day the engine's own lunar rate versus its own centered difference
differs by up to 0.01094″/day, illustrating the truncation/rounding trade-off.
These cross-model comparisons do not establish which rate is correct.

## Historical TT−UT1

The uncorrected Espenak–Meeus polynomial was evaluated at the numeric epochs
of the caller-supplied [USNO historic table](https://maia.usno.navy.mil/ser7/historic_deltat.data)
(656 half-year entries, 1657–1984.5). The table hash was verified before and
after. The maximum absolute model-minus-table difference was **12.6811 s**
at 1686.5; RMS over all entries was 4.5747 s. For 1800–1899.5 the maximum was
1.5791 s; for 1900–1971.5 it was 0.2863 s. The table and polynomial are
historical estimates/reconstructions, potentially based on overlapping data;
their difference is not the actual error or a calibrated uncertainty. The
comparison uses the numeric year labels directly, without the engine's
calendar interpolation.

The existing table API's `MEASURED` label means **not from the prediction
file**, including historical reconstructions; it must not be interpreted as
direct atomic-clock observation at all dates. The local source labels its
third column `error`; its statistical meaning has not been independently
verified. The engine retains its historical Julian-year mapping
`2451545.0 + (year−2000) × 365.25`: compared with Gregorian calendar-year
fractions it differs by up to **3.75 days** in the supplied table. The source
file itself does not specify a calendar mapping; choosing a replacement needs
source confirmation and compatibility review. These clarifications do not
change the numerical lookup or its legacy labels.

## Effect of historical-time assumptions

At six individual July epochs, the engine's explicit UT1 path was evaluated
at a historical model's ΔT and at ± an illustrative old standard-error
estimate from [NASA's 2004 uncertainty discussion](https://eclipse.gsfc.nasa.gov/SEhelp/uncertainty2004.html).
These are historical example estimates, not a modern calibrated guarantee:

| Astronomical year | Illustrative ΔT uncertainty | Approximate change in Moon longitude for ± uncertainty |
| ---: | ---: | ---: |
| 0 (1 BCE) | 265 s | ±130.3″ |
| 500 | 139 s | ±85.3″ |
| 1000 | 54 s | ±28.1″ |
| 1200 | 31 s | ±17.5″ |
| 2500 | 612 s | ±303.1″ |
| 3000 | 1885 s | about ±1022″ |

The 2500/3000 values are extrapolations, not observations. The estimates vary
by date and model; a reported source error is not automatically a 1σ bound
on an interpolated result. This evaluates only the effect of changing ΔT on
geocentric positions; local time-zone history, unknown birth time, Earth
rotation for house angles and model/frame errors remain separate. Explicit
UT1, caller-supplied ΔT, and uncertainty are required; UT1 is not silently
declared to be UTC.

## Reproduction and retained evidence

Commands run from `bindings/node` after its local native build; supply a local
classic DE441 file, its SHA-256, the caller-supplied Swiss module or USNO
table, and new output paths. Reports remain ignored local data.

```text
node scripts/compare-horizons-icrf-rates.cjs <de441> <sha256> <new-report> 0-3000-step-50
node scripts/compare-horizons-rates-by-position.cjs <de441> <sha256> <new-report> 0-3000-step-50
node scripts/diagnose-horizons-rate-consistency.cjs <de441> <sha256> <new-report> 0.005
node scripts/compare-swiss-long-range.cjs <de441> <sha256> <swisseph> <new-report> 0-3000-step-10 0.005
node scripts/audit-historical-deltat.cjs <historic_deltat.data> <sha256> <new-report>
node scripts/audit-historical-time-sensitivity.cjs <de441> <sha256> <new-report>
```

Local SHA-256 values: DE441
`476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`;
USNO table `9f43514119060601a00624a5d9287a91b0b1f5e12bec5209bbabda90b392b70b`;
direct-rate report `53febdac036eff3c545f0d4dc05d4f99adacce0cd40394c8fcb42924ed891dd3`;
position-differencing report `104b774e7d0877d7dcaafd9026211742e7c18e247da03c144ae902d817bc7bf5`;
selected multi-step comparison `369285aa67d165a2445f3829f1a56d388f950bac7b1e71a538a29e49433f9c41`;
Swiss ±0.005-day report `7da7b561792ed884fc36ab12e359d212a473cd8972ff1ede5a4ffb7ee3a565b4`;
Swiss ±0.01-day report `4c5ae8d3611a85dde68f56bdf17b6b939bfed91b3dcf14e5ecb4058910a67dba`;
historical comparison `97918cb218109b7a44f4fa0f041b71c22ae2400b55cc9b7f32497686a02a940c`;
sensitivity report `5d471477a0dd81e016217afe64243fd667c7f260ee6f6f37522b385487dd3241`.
Hashes bind local inputs/results, not the authenticity of a publisher or a
scientific approval. No external data are committed.
