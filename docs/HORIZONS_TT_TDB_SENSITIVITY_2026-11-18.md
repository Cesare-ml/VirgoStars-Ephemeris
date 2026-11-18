# TT–TDB sensitivity in the Horizons planetary holdout

**Status: exploratory model sensitivity; not a physical timing error or MVP
accuracy bound.** This follows the [300-case Horizons holdout](HORIZONS_PLANET_HOLDOUT_2026-11-18.md).
The stellar catalog, server migration and ordinary calculation paths remain unchanged.

## Method

Re-parse the 30 TT epochs and raw Horizons responses from the existing
holdout report, then calculate each of the 300 apparent geocentric ICRF
directions twice with the same local DE441 engine. The first uses the
single-sinusoid TT-to-TDB approximation from the holdout; the second uses
separately executed ERFA `dtdb` at the geocenter. ERFA's input is split into
two Julian-date parts; its output is TDB−TT in seconds. The computed TDB
is then rounded to the single `f64` Julian date accepted by the local engine.
The first calculation is checked against every original holdout result.

ERFA documents `dtdb` as a geocentric Fairhead–Bretagnon model with optional
topocentric terms; setting both observer distances to zero selects the
geocenter. ERFA also allows TT as the date input for this approximation.
See the [ERFA function documentation](https://github.com/liberfa/erfa/blob/master/src/dtdb.c).
Both models are extrapolated for ancient dates. This experiment does not
establish which time transformation Horizons uses internally, or which
extrapolation is physically correct. The direction comparison shares DE441,
and Horizons's printed coordinates have finite precision. TT is not UT1:
this is distinct from uncertainty in historical ΔT and civil time.

## Results

The largest sampled difference between the two modeled TDB−TT offsets is
**0.2731 ms**, at February of astronomical year −9950. After rounding to the
single Julian date passed to the engine, the corresponding difference is
0.2816 ms. Across the 300 cases, the largest great-circle change in the
local direction between the two time models is:

| Target | Largest direction change | Case |
| --- | ---: | --- |
| Moon | 0.000174″ | year −9950, February |
| Other bodies | 0.0000146″ | Venus, year −9950, February |

At the lunar case above, the separation from the printed Horizons ICRF
direction changes from 0.000456″ to 0.000283″. Across all lunar samples the
largest separation instead changes from 0.000524″ to 0.000574″, both in
year −8750, August. Thus the alternative does not uniformly reduce the
observed discrepancy. Across all cases, 75 residuals decrease, 165 increase
and 60 are unchanged; all unchanged cases share the same rounded TDB input.
These counts reflect these particular dates, printed reference values and
implementations, not a model ranking.

The floating-point rounding effect and the offset model both matter at this
resolution. Neither result is a bound across the interval, a validation of
the real ancient TDB–TT difference or evidence for changing the default.
The study does not test absolute ecliptic orientation, planetary velocity,
topocentric events, or epochs before the Horizons service limit.

## Reproduction

Use a caller-installed pyerfa 2.0.1.5 with NumPy 2, the local native Node
addon, classic DE441 and the original hash-checked Horizons holdout report.
No network request is made. The Python helper checks the geocentric result
is independent of the unused topocentric inputs and hashes its reference
files; the Node audit checks the kernel, interpreter, addon, helper and
source before/after. Input response metadata and TT epochs are re-parsed.
External data and generated reports are not tracked.

```text
node bindings/node/scripts/audit-horizons-tt-tdb-sensitivity.cjs KERNEL KERNEL_SHA HORIZONS_REPORT REPORT_SHA PYTHON NEW_REPORT
python -m unittest discover -s scripts -p test_compare_erfa_tdb.py -v
```

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Horizons source SHA-256: `0279176fbb01cad2ca9c47ecf5345b8e964d88a9e2929b2b93a17bbb9395d885`.
- Sensitivity report SHA-256: `ac7c11266c2a28a58d8d0e6736ddab7bf016beebd349d6b1d99c14c813a81b75`.

The report was reproduced byte-for-byte on Windows using Python 3.12.14,
NumPy 2.3.5, pyerfa 2.0.1.5, ERFA 2.0.1 and SOFA 20231011. A local test
also checks the published ERFA `dtdb` example; that is a reference installation
check, not validation of the ancient extrapolation.
