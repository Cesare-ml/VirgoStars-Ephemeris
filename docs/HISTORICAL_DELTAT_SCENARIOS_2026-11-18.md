# Explicit historical Delta T scenarios

**Status: experimental and opt-in.** This provides a practical sensitivity
calculation for the long-range planetary path. It does not select a Delta T
model, validate historical clocks, or enable the main server.

At a caller-specified **UT1 Julian date**, compute the apparent geocentric
ecliptic-of-date state at three Delta T values: `deltaT - uncertainty`,
`deltaT`, and `deltaT + uncertainty`, all in seconds. UT1 is held fixed;
TT and approximate TDB are calculated separately for each sample. The
caller supplies both numbers and a nonempty source label. The label is only
an assertion, not a provenance or accuracy verification.

The three values are **samples, not position bounds or a confidence
interval**. The caller chooses the meaning of the perturbation; the code
does not assume it is one standard deviation or a measured interval.
Intermediate extrema, direction changes, longitude wrapping at 0°/360°,
model error and other uncertainties are not covered by the endpoints. UT1
is not UTC, a calendar, a time zone, or historical local civil time.

## Use

Rust callers can use
`historical_delta_t_scenarios::historical_delta_t_scenarios`, selecting
`ScenarioFrame::PoleBased` or `ScenarioFrame::DirectObliquity`. Node exposes:

```js
const scenarios = engine.experimentalLongTermApparentUt1Scenarios(
  jdUt1, body, deltaTSeconds, uncertaintySeconds, 'caller source'
);
// scenarios.lower, .nominal and .upper each contain jdTt, jdTdb,
// deltaTSeconds and xx = [longitude, latitude, distance, and three rates].
```

The Node alternative `experimentalLongTermDirectObliquityUt1Scenarios`
uses the separately fitted mixed-obliquity frame. Both methods return their
frame label and notes. Angles are degrees, distance AU, and rates per TDB
day. Sun, Moon, Mercury, Venus and the Mars–Pluto system barycenters use
body IDs 0–9. The model includes light-time, solar deflection, annual
aberration and extrapolated nutation; its long-range accuracy is not
established. The one-term TT-to-TDB approximation is unchanged.

All three samples must succeed: an out-of-range or failed sample returns
`E_CALC` with the failing sample, rather than a partial result. Invalid
input, negative perturbation, non-finite or unrepresentable conversions, and
a nonzero perturbation that rounds to the same TT or TDB Julian date return
`E_RANGE`. Zero is allowed and produces three identical samples. Unsupported
body IDs return `E_UNSUPPORTED_BODY`. The Rust API returns corresponding
structured error variants. No implicit fallback is applied.

## Illustrative local result

With the caller-supplied classic DE441 and proleptic Gregorian July 1 of
astronomical year −12999 (13000 BCE), take a **hypothetical** Delta T of
197.95 hours and a one-hour perturbation, in the mixed frame. For the Moon:

| Scenario | Delta T | Longitude |
| --- | ---: | ---: |
| Lower | 196.95 h | 215.314043° |
| Nominal | 197.95 h | 215.904424° |
| Upper | 198.95 h | 216.494739° |

The one-hour increase changes this sample by about 35.4 arcminutes. These
numbers illustrate sensitivity; they are neither an estimate of actual
historical uncertainty nor a validation of the ancient frame.

## Verification and remaining work

Pure Rust tests check conversion, zero and negative perturbations, overflow,
and distinct f64 epochs. With the local DE441 file, the Node tests compare
each sample to the existing individual UT1 call in both frames, including
ancient and modern dates, Sun through Pluto at calendar endpoints, zero
perturbation, an out-of-range upper sample and a lunar 360°→0° crossing.
They check implementation consistency, not independent physical accuracy.

Further work includes independently validated historical-time models and
their stated applicability, a policy for the caller's chosen uncertainty,
more extensive frame validation, and explicit consumer integration. The
historical ΔT ambiguity cannot be resolved by evaluating more decimal places.
