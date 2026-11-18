# Sensitivity of planetary directions to ancient nutation arguments

**Status: exploratory model sensitivity; not a physical accuracy bound.**
This follows the [nutation component audit](ANCIENT_NUTATION_ARGUMENT_AUDIT_2026-11-18.md)
and uses the previously recorded [Swiss fixed-axis grids](SWISS_ANCIENT_FRAME_DECOMPOSITION_2026-11-18.md).
No production calculation or default is changed. Stars and main-server migration remain paused.

## Method

Take the **same** apparent geocentric ICRS directions from each stored Swiss
2.09.03/DE441 grid and project them twice through the local experimental
Vondrák mean axes and frame bias. The only changed input is the argument
formulation for the local 77-term nutation: full IERS polynomial arguments
versus standard linear arguments. For the true equator both nutation components
enter the rotation; for the pole-based true ecliptic the longitude component
enters the rotation. Both approaches use the same fixed direction and the
same remaining orientation model. The resulting great-circle separations are
reported in arcseconds; they are not differences in longitude alone.

There are 2,760 baseline and 2,720 interleaved body/date cases, comprising
Sun, Moon, Mercury, Venus and Mars–Pluto system barycenters, four seasons in
each of 137 distinct astronomical years, −12999 through 3999. The stored
directions were calculated at approximately matched TT and TDB, not historical
UT1; see the source grid documentation for reference flags and corrections.
Each source hash, date, flag and input direction is checked. The full-argument
projection must agree with the independently executed existing orientation
diagnostic within `1e-7″`, a numerical consistency threshold. It is not a
physical tolerance. Both diagnostics use the same Rust orientation functions.

## Results

| Grid | Cases | Maximum equatorial separation | Maximum ecliptic separation |
| --- | ---: | ---: | ---: |
| Baseline | 2,760 | 7.490063″ | 7.438182″ |
| Interleaved | 2,720 | 8.299158″ | 8.264810″ |

In the interleaved grid the maximum equatorial separation occurs for the
Uranus system barycenter in July of astronomical year −12875; the maximum
ecliptic separation occurs for the Sun in the same month. These are two
different coordinate frames, not two independent measurements of a position.
Across both grids 1,000 cases are earlier than year −9998. Both maxima in the
table occur in that earlier subset. Separate maxima need not occur for the same
body or date.

This establishes the *sampled effect of the argument choice* with other
inputs held fixed. It does not say that either formulation is physically
correct for the ancient Earth, that the discrepancy is a planetary orbit
error, or that these maxima hold between sampled dates. It is not a validation
of absolute apparent positions, precession, time-scale uncertainty, speeds,
rise/set or a production readiness claim. Both directions originate from the
same DE441 solution and Swiss computation; no independent orbit reference is
introduced. See the component audit for the separate ERFA comparison.

## Reproduction

Build both examples and supply an existing fixed-grid report, its SHA-256,
and a new output path. The program validates the source grid and every
full-argument projection, checks the source and executables before/after,
and never overwrites a report. No kernel or Swiss package is needed to replay
this projection from the locally retained report. The source data are not
bundled with the repository.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_nutation_direction_impact --example diagnose_horizons_orientation_components
node bindings/node/scripts/audit-nutation-direction-impact.cjs FIXED_GRID_REPORT REPORT_SHA NEW_PROJECTOR EXISTING_PROJECTOR NEW_REPORT
```

- Baseline source SHA-256: `f055b695016353838a168ab32ca34ea99fcdc385e1e0166d3330ca7450eb2264`.
- Interleaved source SHA-256: `7dd00e18c3a3935446237bd357a23532edd87fe107ffc7ac435dc77436db942c`.
- Baseline output SHA-256: `a94a083f750f09c174495f6800c80ed8e86161ba0432be48bb59195798d4bf4b`.
- Interleaved output SHA-256: `b88edc404f8343a878141c0abc6e517cbfcadc51c5f45043b8ca9e9e4ddca638`.

Both reports were reproduced byte-for-byte on the same Windows build. Source
reports record the DE441 and Swiss provenance; they remain locally retained,
not tracked. Further work requires a physically motivated long-term orientation
choice, independent reference evidence and agreed MVP tolerances.
