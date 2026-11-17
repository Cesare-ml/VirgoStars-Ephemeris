# Ancient ecliptic convention and ΔT comparisons — 2026-11-17

**Experimental alternative, not a precision or historical-time guarantee.**
This continues the [13000 BCE–3999 CE diagnostic](PLANET_VALIDATION_13000BCE_3999_2026-11-17.md).
Stellar work and main-server migration remain paused. The existing pole-based
method and normal APIs retain their behavior; this alternative is opt-in.

## Two separately fitted obliquities

[Vondrák, Capitaine and Wallace (2011)](https://www.aanda.org/articles/aa/pdf/2011/10/aa17274-11.pdf)
provide independently fitted series for the equator and ecliptic poles and,
in Eq. 10 / Table 3, for obliquity ε_A. These parameterizations need not agree
exactly, especially far from J2000. In a first local probe for July of
astronomical year −12999, the angle between the pole series was about
23.870997°, while the direct ε_A series was about 23.883741°. The local Swiss
`ECL_NUT` mean-obliquity output was about 23.883741°. This numerical
observation identifies a model/convention difference; it does not establish
which historical pole is physically more accurate or explain Swiss internals.

The new explicit Rust/Node candidate uses the existing pole-derived equator and
equinox, rotates by the independently fitted direct ε_A series, and applies
the same extrapolated nutation and apparent corrections as the earlier path.
This is a **mixed convention**: its ecliptic pole is not the original
pole-series ecliptic. It is not presented as an independent orbit solution,
fully self-consistent refit, or established accuracy improvement. The rate
uses the time derivative of the selected matrix; it is not copied from Swiss.
The original pole-based experimental methods and the modern `calcTdb` /
`calcUt` paths are unchanged.

Node exposes `experimentalLongTermDirectObliquityTdb(jdTdb, body)` and
`experimentalLongTermDirectObliquityUt1(jdUt1, body, deltaTSeconds,
uncertaintySeconds, source)`. The latter requires caller-asserted ΔT and
uncertainty; it does not derive a position-error bound or treat UT1 as UTC.
Both include the mixed-convention warning in result notes. The model uses
the published Eq. 10 coefficients rather than copying another implementation.

On 2026-11-18 an additional check compared the direct-obliquity series with
the independently published ERFA `t_obl06` IAU 2006 reference at TT JD
2454388.5: the difference was about −0.0000009 arcsecond. The two are distinct
models that agree near J2000; this modern check does **not** validate either
the mixed ecliptic or obliquity in antiquity. ERFA's published fixture is at
<https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c>.

## Reproducible same-DE441 comparisons

All cases use the local Swiss 2.09.03 JPL mode, geocentric apparent ecliptic
of date, Sun/Moon/Mercury/Venus and the Mars–Pluto system barycenters. Inputs
are TDB here and approximately corresponding TT in Swiss. Rates compare with
a fourth-order derivative of Swiss **positions**, using 0.02 and 0.04 day
offsets; they do not compare against Swiss's reported speed. The original
July grid was used in the prior diagnosis; the other seasons and interleaved
years were not part of that grid. They use the same reference implementation
and are not an independent physical validation.

| Dataset | Cases | Pole-based max longitude / latitude | Direct ε_A max longitude / latitude | Pole / direct max latitude-rate difference |
| --- | ---: | ---: | ---: | ---: |
| Baseline years, Jan/Apr/Jul/Oct | 2,760 | 8.844″ / 45.873″ | 0.002107″ / 0.000308″ | 9.380 / 0.001458 ″/day |
| Interleaved years, Jan/Apr/Jul/Oct | 2,720 | 7.825″ / 44.223″ | 0.002230″ / 0.000323″ | 9.278 / 0.001566 ″/day |

On these grids the direct-convention maximum longitude-rate difference from
the Swiss position derivative was 0.021064″/day (baseline) and 0.015057″/day
(interleaved), both for the Moon. The direct candidate's maximum longitude
rate minus the derivative of its *own* positions was 0.001619″/day in one
baseline Uranus case; this and the finite-difference step, numerical epoch
precision, time-scale approximation and extrapolated nutation warrant further
investigation. These maxima are sampled differences, not product tolerances,
continuous bounds, independent orbital accuracy or proof of historical truth.

The baseline Uranus outlier is only 0.267° from the Sun's apparent ICRF
center. A separate six-step diagnostic changes its own-position derivative
residual from 0.001618″/day at a 0.02-day half-step to approximately
0.001069″/day at 0.001–0.002 days. This demonstrates numerical sensitivity;
it does not identify its cause or establish an error bound. Near-solar cases
and the apparent-correction derivatives need further independent analysis.

On 2026-11-18 a component diagnostic traced the ICRF right-ascension rate of
this same Uranus case (TDB JD −1474538). At a binary-exact 1/1024-day
half-step, reported rate minus a fourth-order derivative of each stage's own
positions was approximately −0.0000012″/day before solar deflection,
+0.0010643″/day after deflection, and +0.0010655″/day for the full
experimental ICRF result. These are internal consistency checks in fixed ICRF
axes, not independent physical reference measurements. The unrefined stages
and the experimental full stage have different light-time rounding treatments.

For the deflection contribution alone, a two-point derivative with the
implementation's 1/256-day half-step agrees with its reported contribution to
about 0.00000004″/day, while using 1/1024 day differs by about 0.000999″/day.
This isolates the dominant residual in this particular sample to sensitivity
of the deflection-rate stencil. Two days later, the experimental full-stage
residual at 1/1024 day is about 0.00000022″/day; 100 days later it is about
−0.00000005″/day. This is diagnostic evidence, not a universal error bound:
the underlying first-order model itself warns about rays close to the Sun.
No production rate algorithm is changed pending broader tests of any remedy.
An additional synthetic test compares a source-at-infinity case with the
analytic derivative of the cotangent deflection law; reducing the centered
half-step by four reduces that numerical error by more than a factor of 14.
That confirms the expected convergence in this synthetic geometry only.

## Ancient ΔT: compare models without declaring a winner

At a fixed proleptic Gregorian UT1 instant in astronomical year −12999, the
uncorrected [NASA 2006 parabola](https://eclipse.gsfc.nasa.gov/SEcat5/deltatpoly.html)
and the [Stephenson–Morrison–Hohenkerk 2016 parabola](https://www.cambridge.org/core/services/aop-cambridge-core/content/view/47227DA52FF4120A949BF70140CF1D23/S1743921319003971a.pdf/eclipses-and-the-earths-rotation.pdf)
differ by about **11,161 s (3.10 h)** in this diagnostic. Propagating those
two values through the direct-obliquity candidate changes the Moon's ecliptic
longitude by about **110 arcminutes**. Both formulae here are extrapolated
outside their observational/calibration ranges: NASA specifies its polynomial
for −1999..3000, and the 2016 observations cover approximately 720 BCE–2015.
The 2016 parabola is not the observational spline. The formulas can share
historical evidence, and their difference is **not an uncertainty interval**.

For six fixed TT dates from 9999 BCE through 720 BCE, the local Swiss ΔT and
[Horizons quantity 30](https://ssd.jpl.nasa.gov/horizons/manual.html) were
compared at the corresponding UT1 estimates. At the earliest date they differ
by about **13,340 s (3.71 h)**; at 720 BCE by about 67 s. Horizons's pre-1962
quantity is approximately TT−UT1 and is itself a reconstruction; the sources
can share observations. Horizons cannot provide this check at 13000 BCE.
Neither difference is an error bar or establishes the better model. No new
ΔT default, accuracy claim, or historical local-time conversion is introduced.

A denser 39-date fixed-TT comparison in 2026-11-18 samples 9999–720 BCE at
roughly 250-year intervals. Swiss minus Horizons ranges from 67 to 13,340 s
in these samples (e.g. 4,057 s in astronomical year −5000). Both use
reconstructions that may share evidence; the range is not an error bar.

## Evidence and reproduction

From the repository root, after rebuilding `bindings/node`, use a separately
supplied classic DE441, installed Swiss module, the existing kernel SHA-256,
and fresh output files:

```text
node bindings/node/scripts/compare-ancient-obliquity.cjs <de441> <sha256> <swisseph> <new-output> baseline
node bindings/node/scripts/compare-ancient-obliquity.cjs <de441> <sha256> <swisseph> <new-output> interleaved
node bindings/node/scripts/audit-ancient-delta-models.cjs <de441> <sha256> <swisseph> <new-output>
node bindings/node/scripts/compare-horizons-ancient-deltat.cjs <de441> <sha256> <swisseph> <new-output>
node bindings/node/scripts/audit-ancient-rate-outlier.cjs <de441> <sha256> <swisseph> <baseline-report> <baseline-sha256> <new-output>
node bindings/node/scripts/compare-horizons-ancient-deltat.cjs <de441> <sha256> <swisseph> <new-output> dense
cargo run --offline --locked -p virgostars-ephemeris-core --example diagnose_near_sun_rates -- <de441> -1474538
```

The Horizons comparison command accesses Horizons. Reports and kernels remain ignored local
files. The input kernel is checked before and after each run; local SHA-256s:

- Baseline: `1ae8b250f1c7dd3797432f4c10c0a3f77a9dd26154afd4e88c509289f2f694e6`.
- Interleaved: `213b82c94e0ab0a24d3c0ce24c9ad2f1e02d28e55278e08546827504dfe223b9`.
- Three-model UT1 scenario: `98d66a14fab317db575c4361c592a4aa1b93ebf1f5c1ee2787c0ec31987273fb`.
- Fixed-TT Horizons comparison: `b71faf833ae877642a20890cfb84277684580e94ed69d80539de2b0c69f47ab8`.
- Near-solar rate sensitivity: `35bf2414cfd7ee20ee049d2dc34c1d7b7c068f1389034bb8c211c29d99750c05`.
- Dense fixed-TT Horizons comparison: `823866e68854081667c06591d40a7664254e63cde4ea2950c842505c77ba6342`.

Continue with independent orientation reference checks, numerical rate
stability, agreed frame/time tolerances and a sourced uncertainty policy before
promoting either long-range convention or extending dependent lunar events.
