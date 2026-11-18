# ERFA long-term mean axes and planetary direction conformance

**Status: sampled implementation conformance; not physical validation.**
This extends the earlier [six-epoch comparison](LONG_RANGE_AND_SWISS_ALIASES_2026-11-17.md)
and the [ancient direction sensitivity study](ANCIENT_NUTATION_DIRECTION_IMPACT_2026-11-18.md).
The ordinary and experimental calculation paths are unchanged. Stars and main-server migration remain paused.

## Method

At explicit TT dates, compare the local experimental Vondrák mean-equator
matrix with separately executed ERFA `ltp`, and the local mean-ecliptic matrix
with a matrix reconstructed from ERFA's `ltp` equinox and `ltpecl` pole. The
local code and ERFA share the same published coefficient data: this is a
**numerical conformance test of implementations of the same model**, not an
independent measurement of ancient Earth orientation. The matrices transform
J2000 mean equator/equinox axes into mean axes of date. They contain no
nutation or frame bias. The implementation also checks ERFA `ltpequ` against
the `ltp` pole, proper rotation determinants, orthogonality and finite values.
ERFA describes the matrix convention and model in its
[long-term precession documentation](https://github.com/liberfa/erfa/blob/master/src/ltp.c).

The first grid contains 3,887 distinct TT epochs from astronomical year
−12999 through 3999, with seasonal, selected monthly and daily samples as
described in the [nutation audit](ANCIENT_NUTATION_ARGUMENT_AUDIT_2026-11-18.md).
Separately, project the **same** 5,480 stored apparent geocentric Swiss/DE441
ICRS directions from the two prior [fixed-axis planetary grids](SWISS_ANCIENT_FRAME_DECOMPOSITION_2026-11-18.md)
through each pair of mean matrices. Apply the same ERFA IAU 2006 J2000 bias
to both projections; the bias itself is not being validated. The source
directions use approximately matched TDB and TT, not historical UT1. They
comprise the Sun, Moon, Mercury, Venus and Mars–Pluto system barycenters.

The axis difference is the greatest angular separation among corresponding
matrix rows, not a bound on physical orientation. The planetary direction
separation is measured after the mean rotations, not a full apparent of-date
coordinate comparison. Before evaluating any data, the test sets a maximum
component difference of `2e-14` and axis/direction separation of `1e-8″`
as **implementation conformance gates**, not scientific error tolerances.

## Results

| Sample | Cases | Largest equatorial axis difference | Largest ecliptic axis difference |
| --- | ---: | ---: | ---: |
| TT date grid | 3,887 | `2.80e-10″` | `2.44e-10″` |
| Planet baseline epochs | 276 | `2.31e-10″` | `2.28e-10″` |
| Planet interleaved epochs | 272 | `2.17e-10″` | `1.88e-10″` |

The largest individual matrix component difference on the TT date grid is
`1.12e-15`. Across the 5,480 planetary directions, the largest equatorial
and ecliptic projection differences are `2.70e-10″` and `2.41e-10″`,
respectively. These extremely small values describe agreement of two
implementations with shared coefficients and floating-point arithmetic;
they **do not establish microarcsecond physical accuracy**. Neither the
nutation, actual Earth pole, orbital accuracy, topocentric coordinates,
velocities, historical ΔT nor unsampled dates are independently validated.
Prior Horizons and Swiss comparisons remain model comparisons and cannot by
themselves select the physically best convention.

## Reproduction

The audit requires caller-installed pyerfa 2.0.1.5 with NumPy 2, the Rust
example and a previously recorded local input report with its SHA-256. It
validates shapes, dates, matrices and direction groups, checks source,
projector, interpreter, script and reference-package hashes before/after,
and refuses to overwrite an output. ERFA, Swiss, kernels and reports remain
outside the tracked repository.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_long_term_mean_axes
python scripts/audit_erfa_long_term_axes.py INPUT_REPORT INPUT_SHA PROJECTOR NEW_REPORT
python -m unittest discover -s scripts -p test_audit_erfa_long_term_axes.py -v
```

- Date-grid input SHA-256: `903d613049975c2c29abd1e64bd3c92ba19b47506b17118cdfdc11e689ab4bdd`.
- Baseline input SHA-256: `f055b695016353838a168ab32ca34ea99fcdc385e1e0166d3330ca7450eb2264`.
- Interleaved input SHA-256: `7dd00e18c3a3935446237bd357a23532edd87fe107ffc7ac435dc77436db942c`.
- Date-grid output SHA-256: `07048be91d57e182df4ad2c5a4e33aa6c35fad37d650dd789c7d41e98cbc16a2`.
- Baseline output SHA-256: `7acb4f2bf62c0397372334e13512b7064b3591d6615a56f31ae54eaa3137ce39`.
- Interleaved output SHA-256: `5fc67df8f9a9f3f0de6be554452461521ce1f4578cd473f2006d6b1e318ca3b2`.

All three outputs reproduced byte-for-byte on the same Windows build using
Python 3.12.14, NumPy 2.3.5, pyerfa 2.0.1.5, ERFA 2.0.1 and SOFA 20231011.
