# Ancient nutation: arguments and model comparisons

Follow-up: [sampled planetary direction sensitivity](ANCIENT_NUTATION_DIRECTION_IMPACT_2026-11-18.md).

**Status: exploratory numerical/model comparison, not an estimate of the
physical ancient Earth orientation or an MVP accuracy bound.** Stars and the
main-server migration remain paused. This continues the earlier
[long-range orientation work](LONG_RANGE_AND_SWISS_ALIASES_2026-11-17.md)
and [Swiss ecliptic frame decomposition](SWISS_ANCIENT_FRAME_DECOMPOSITION_2026-11-18.md).

## Models and sample

The existing experimental path evaluates the 77-term 2000B series with full
IERS 2003 polynomial fundamental arguments. It is **not standard IAU 2000B**.
For this study a separately named Rust diagnostic evaluates the same 77 terms
using the standard *linear* arguments; no normal calculation selects it.
Compare both longitude nutation Δψ and obliquity nutation Δε against
independently executed ERFA `nut00a` and `nut00b` and Swiss `SE_ECL_NUT`.
The local 77-term coefficients and standard offsets are derived from ERFA's
published data, so the linear comparison is an implementation-conformance
check with shared coefficient provenance, not independent physical evidence.

The input is explicit TT, with proleptic-Gregorian calendar dates and
astronomical year numbering. There are 3,887 distinct TT epochs: four
seasonal samples in 681 years spaced mostly 25 years apart, additional
monthly samples in 13 selected years, and 365 daily samples in each of years
−12999, 2000 and 3999. Overlapping dates are counted once. There are 861
samples in years earlier than −9998. This remains a sparse sample over the
full interval; it is neither historical UT1 nor civil time.

ERFA documents the standard 2000B implementation as a modern abridged
model, with its stated pole accuracy applying to 1900–2100; its 2000A
implementation also omits free-core nutation. See the [ERFA 2000B](https://github.com/liberfa/erfa/blob/master/src/nut00b.c)
and [2000A](https://github.com/liberfa/erfa/blob/master/src/nut00a.c)
documentation. Evaluating either far outside modern epochs is an extrapolation,
not observational validation. Swiss's [SE_ECL_NUT contract](https://www.astro.com/swisseph/swephprg.htm)
supplies mean/true obliquity and the two nutation components; its results are
also computed models. No kernel is needed for this nutation-only comparison.

## Results

All values below are maximum **absolute component differences** in
arcseconds across the 3,887 sampled epochs. Δψ and Δε are nutation components,
not automatically errors in a particular body's longitude or latitude.

| Comparison | Δψ | Δε |
| --- | ---: | ---: |
| Local full-argument 77 terms − ERFA 2000A | 0.002370″ | 0.001592″ |
| Local full arguments − local linear arguments, same 77 terms | 8.292388″ | 4.845069″ |
| Local linear arguments − ERFA 2000B | 0.000000000000013″ | 0.0000000000000063″ |

The largest full-versus-linear Δψ difference occurs in year −12800 January;
the Δε maximum occurs in year −12675 October. Because the two local variants
use the same terms and differ in their arguments, that controlled comparison
isolates the numerical effect of the argument choice on these samples. The
linear variant agrees with ERFA 2000B on this grid; its `1e-8″` conformance
gate checks implementations of the same series, not physical accuracy.
The much smaller differences from ERFA 2000A do not prove either extrapolated
model represents the actual ancient Earth.

Relative to Swiss 2.09.03, local full-argument Δψ is approximately −0.000135″
and Δε approximately +0.000388″ at all these samples. Those values match the
fixed offsets explicitly applied by the local 2000B evaluator; subtracting
them leaves maximum residuals about `5.12e-10″` and `2.33e-10″` respectively.
This is an observed numerical relationship between outputs, not a claim about
Swiss's internal implementation or independent physical accuracy.

The existing full-argument output was also compared before and after adding
the diagnostic: all 3,887 sampled values were identical. The standard
`iau2000b_nutation` guard and normal calculation paths remain unchanged.
Selecting a production long-term orientation still requires an accepted
model, reference conventions, time policy and tolerances.

## Reproduction and evidence

Use caller-installed **pyerfa 2.0.1.5** with NumPy 2 or newer importable by
the supplied Python executable; the package used locally was installed in an
ignored folder. Build `diagnose_experimental_nutation`, then use new report
paths. The ERFA generator receives TT values over stdin and returns both
components; the orchestration checks each epoch and hashes the Python,
projector, helper and reference files before/after. Swiss is optional and is
provided separately; its package/wrapper/binary and ERFA source report are
also hash-checked before/after. No ERFA, Swiss or reference output is bundled.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_experimental_nutation
node bindings/node/scripts/audit-erfa-nutation-grid.cjs PYTHON PROJECTOR NEW_ERFA_REPORT
node bindings/node/scripts/audit-swiss-nutation-grid.cjs SWISSEPH_PACKAGE ERFA_REPORT ERFA_REPORT_SHA NEW_SWISS_REPORT
```

- ERFA comparison report: `903d613049975c2c29abd1e64bd3c92ba19b47506b17118cdfdc11e689ab4bdd`.
- Swiss comparison report: `44be331001f963c6bb3a5c0700e901b725912418b66a23188231cd77f04dd8e1`.
- Windows run: Python 3.12.14, NumPy 2.3.5, pyerfa 2.0.1.5, ERFA 2.0.1,
  SOFA 20231011 and Swiss 2.09.03.

Both reports were reproduced byte-identically. Local tests check the modern
ERFA anchor, the diagnostic guard, grid uniqueness, component summaries,
flag/coordinate validation and failure paths. Results do not validate
long-range precession, the complete apparent position, UT1 uncertainty,
velocities or production integration.
