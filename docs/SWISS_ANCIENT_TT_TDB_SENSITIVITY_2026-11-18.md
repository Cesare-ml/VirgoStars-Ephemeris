# TT–TDB sensitivity before and within the Horizons service interval

Follow-up: [measured DE441 epoch quantization](DE441_ANCIENT_EPOCH_QUANTIZATION_2026-11-18.md).

**Status: exploratory sampled sensitivity, not physical validation or an
MVP accuracy bound.** This extends the [Horizons timing study](HORIZONS_TT_TDB_SENSITIVITY_2026-11-18.md)
to the earlier interval using the [recorded Swiss/DE441 grids](SWISS_ANCIENT_FRAME_DECOMPOSITION_2026-11-18.md).
The stellar catalog, main-server migration and ordinary calculation paths remain unchanged.

## Method

The two stored fixed-axis grids contain 5,480 Sun, Moon and planetary
body/date combinations at 548 distinct dates in astronomical years −12999
through 3999, including 1,000 cases earlier than year −9998. The two grids
use disjoint years and the same Swiss and DE441 implementations. Their
hash-checked rows contain Swiss apparent geocentric ICRS directions at
approximately matched TT, plus local results at the original TDB dates.

For each stored TT instant evaluate the prior single-sinusoid TT-to-TDB
approximation and the separate geocentric ERFA `dtdb` model. Recompute local
apparent directions at each resulting TDB, keeping the kernel and corrections
fixed. Compare the two directions in ICRF and in the local experimental
pole-based ecliptic-of-date frame. Separately compare each ICRF direction
against the **same stored Swiss direction**. Original TDB results must
reproduce their stored Swiss comparisons; requested source flags, dates,
directions and provenance are checked before proceeding.

The original Swiss grid was generated at fixed TDB with an approximate TT
derived from it. Reversing the approximation reproduced the original
single-`f64` TDB in all sampled cases; this is a sampled numerical observation,
not a general identity. ERFA receives TT in two parts, but the resulting TDB
is rounded to a single `f64` Julian date by the local engine. See the earlier
study and [ERFA's `dtdb` documentation](https://github.com/liberfa/erfa/blob/master/src/dtdb.c)
for the geocentric model and its limitations.

## Results

| Sample | Cases | Largest modeled offset difference | Largest ICRF direction change | Largest local ecliptic direction change |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 2,760 | 0.390920 ms | 0.000228783″ | 0.000228783″ |
| Interleaved | 2,720 | 0.395607 ms | 0.000224279″ | 0.000224280″ |

All these maxima lie in the pre-Horizons subset. The largest modeled offset
difference occurs in April of year −11125; the largest direction change is
for the Moon in April of year −11500. The largest difference between the
two rounded TDB inputs is 0.402332 ms. For each grid, the two model runs
produced identical rounded TDB values in 240 and 370 body/date cases,
respectively. Great-circle ICRF and local ecliptic changes differ by at most
about `1.44e-9″` on these samples; this numerical comparison does not
independently validate the ecliptic orientation.

Relative to the stored Swiss ICRS directions, the largest lunar separation
in the interleaved grid is about 0.002096″ with the simple conversion and
0.002163″ with ERFA; these occur at different dates. In that grid 297
residuals decrease, 2,053 increase and 370 are unchanged. Such comparisons
do not establish the Swiss internal conversion or rank the physical models.
Both time models are extrapolated, both position calculations share DE441,
and no actual ancient Earth orientation or time measurement is introduced.

These are sampled angular differences, not limits for unsampled dates or
velocities. They do not validate historical UT1/ΔT, absolute ecliptic axes,
topocentric events, or complete MVP precision, and do not justify a default
change. Date numbering is astronomical and the calendar is proleptic Gregorian.

## Reproduction

Build the native Node addon. Supply the classic DE441 file, one recorded
Swiss grid and its SHA-256, a Python executable with pyerfa 2.0.1.5 and NumPy
2, and a new output path. The helper computes ERFA time offsets at the
geocenter. No Swiss package or network access is needed when replaying the
recorded grid. The audit checks kernel, input report, addon, Python and helper
hashes before/after; the helper checks reference package hashes. Kernel,
Swiss data, ERFA and generated reports are not tracked.

```text
node bindings/node/scripts/audit-swiss-ancient-tt-tdb-sensitivity.cjs KERNEL KERNEL_SHA SWISS_GRID GRID_SHA PYTHON NEW_REPORT
```

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Baseline grid SHA-256: `f055b695016353838a168ab32ca34ea99fcdc385e1e0166d3330ca7450eb2264`.
- Interleaved grid SHA-256: `7dd00e18c3a3935446237bd357a23532edd87fe107ffc7ac435dc77436db942c`.
- Baseline report SHA-256: `e9498bc98d4bd8eed0eb236cfe55f0c9c5fb9077da75bdbff1ebd601242217bc`.
- Interleaved report SHA-256: `de5fe4ff411ee168c96c022164ee9b1291e8f48e8d39cc7e3984a4e6872e6fa9`.

Both outputs reproduced byte-for-byte on the same Windows build using Python
3.12.14, NumPy 2.3.5, pyerfa 2.0.1.5, ERFA 2.0.1 and SOFA 20231011.
