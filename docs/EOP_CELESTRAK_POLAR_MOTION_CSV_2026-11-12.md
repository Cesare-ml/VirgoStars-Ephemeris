# Offline CelesTrak EOP CSV polar-motion reader — 2026-11-12

**Status:** experimental Rust component in a **draft PR** against `dev/ephemeris`.
This component parses independently supplied data and interpolates `xp` and
`yp` only. It does **not** ship a recent EOP dataset or download one.

## Objective

The existing IERS polar-motion work (#84, included in draft integration #89)
requires externally obtained pole coordinates `xp`, `yp`, in **radians**.
Draft PR #100 supplies the optional conventional approximate TT-based TIO
locator `s′`, and draft integration #105 combines it with the polar matrix.
None of those PRs contains EOP data acquisition or an offline input parser.

This change is a **standalone, optional EOP input component**. It consumes
the specific [CelesTrak EOP CSV layout](https://celestrak.org/SpaceData/EOP-format.php),
which CelesTrak documents as aggregating observed and predicted EOP
parameters from external sources, including IERS and leap-second
information. It does not duplicate the polar-motion matrix or
TT/UT1/TDB algorithms from other pending PRs.

The **exact required header** is:

```csv
DATE,MJD,X,Y,UT1-UTC,LOD,DPSI,DEPS,DX,DY,DAT,DATA_TYPE
2000-01-01,51544,0.043282,0.377909,0.3553880,0.0009536,-0.050263,-0.002471,-0.000005,-0.000050,32,O
```

The above record, including its observed `O` flag, is copied **only
as a published numerical test fixture** from CelesTrak's format
documentation. It is a dated example, **not** a current EOP sample
for the user's location or observation. No CelesTrak source code is copied.

## Units and epoch

| Field | Meaning |
| --- | --- |
| `DATE` | ISO Gregorian **UTC** date of the daily 0h UTC sample |
| `MJD` | UTC-labeled integer Modified Julian Day for the same date |
| `X`, `Y` | Polar-motion x/y in **arcseconds**; converted to radians |
| `UT1-UTC` | **seconds**, preserved *at 0h UTC only* |
| `DAT` | `TAI−UTC` **seconds**, preserved *at 0h UTC only* |
| `DATA_TYPE` | `O` observed or `P` predicted (explicitly distinguished) |

The parser validates all 12 CSV fields, checks that ISO Gregorian dates
match the given MJD, rejects malformed/nonfinite values, requires
strictly increasing MJD days and rejects implausibly large polar
coordinates (greater than 0.01 rad in magnitude). This numerical
bound matches the independent draft polar-motion API's input guard;
it is **not** an IERS physical measurement bound.

```text
xp_rad = X_arcseconds × π/(180×3600)
yp_rad = Y_arcseconds × π/(180×3600)
```

No default EOP epoch, TT offset, UT1 epoch or last-known-value
fallback exists. The caller provides the source label, text, UTC MJD
day and the fraction of that **actual UTC civil day**, in [0,1).
Interpreting UTC leap-second day duration is the caller's responsibility.

## Rust usage

```rust
use virgostars_ephemeris_core::eop_polar_csv::CelestrakEopPolarCatalog;

// The caller separately downloads/reads an approved EOP CSV
// snapshot, and retains its release date, provenance and license.
# fn example(approved_snapshot_csv: &str) -> Result<(), Box<dyn std::error::Error>> {
let catalog = CelestrakEopPolarCatalog::parse_csv(
    "CelesTrak EOP CSV, dated snapshot verified by application",
    approved_snapshot_csv,
)?;
let pole = catalog.interpolate_observed(51544, 0.25)?;
println!("xp={}, yp={} rad", pole.xp_rad, pole.yp_rad);
# Ok(())
# }
```

The parser accepts the documented CSV, optional UTF-8 BOM,
CRLF and empty lines. It rejects alternate CSV column orders or
arbitrary quoted CSV syntax rather than guessing a format.

`interpolate_observed` allows only **observed** `O` rows, by
default. `interpolate(..., EopPredictionPolicy::AllowPredicted)`
permits a deliberate choice to include `P` forecast rows and returns
`EopPolarQuality::IncludesPrediction` if either endpoint is predicted.
The output records this quality; a prediction must never masquerade
as observed data.

For nonzero day fractions the algorithm uses **linear interpolation**
between exactly two adjacent **0h UTC** days; gaps, uncovered days,
missing next samples and queries outside range are typed errors.
At fraction 0 the exact row is used. No extrapolation is allowed.
Linear interpolation is an engineering choice that requires
independent error-budget validation for demanding applications,
especially around model/series changes. Source freshness and
reference-frame realization are **not** inferred from the label.

### Important leap-second boundary

`UT1-UTC` and `DAT` are **not** interpolated or converted by this
component. For example, near a leap second, UT1−UTC can jump by
about one second while the physical Earth rotation is smooth.
Interpolating raw DUT1 across that step would be misleading.
The output deliberately contains **only** `xp`, `yp`, UTC day and
quality. The caller must separately implement and validate a
UTC/TAI/TT/UT1 timeline that handles leap-second schedules and
time-equivalence before combining EOP values with ERA(UT1)
and the TT-based TIO locator s′.

## Tests and numerical references

Eight offline test groups validate:

1. Published **CelesTrak** `2000-01-01` row, including x/y
   arcsecond-to-radian conversion, DUT1 metadata, DAT, MJD and `O`.
2. Precisely defined half-day interpolation from the published row
   to an explicitly **synthetic** following day.
3. Predicted `P` rejection without opt-in; authorized output marked
   `IncludesPrediction`.
4. Missing adjacent days, last-day fraction and out-of-coverage.
5. Invalid fractions (NaN, infinities, negative, >=1).
6. Source label, header, chronological ordering, duplicate dates, BOM/CRLF.
7. Bad calendar dates, inconsistent MJD, nonfinite input and wrong units.
8. Synthetic 2016-12-31→2017-01-01 example preserving different
   per-day `DAT` and DUT1 metadata **without interpolating DUT1**.

The first fixture validates a **published format example**, not
independent scientific parity with recent IERS EOP observations.
All other interpolation fixtures are mathematical and/or synthetic.
Before production, an externally obtained real EOP snapshot must
be checked for legal provenance, date coverage, quality, reference
frame, latency and agreement with independently supplied IERS values.

## Next integration boundary

A caller can explicitly obtain `xp_rad`, `yp_rad` from the EOP
catalog and pass them into the unmerged `polar_motion` module or
the optional `tio_polar_motion` wrapper (#105), **only after**
ensuring they correspond to the same physical epoch and TT/UT1 model.

This PR **does not** change #84/#89/#100/#105 or add their unmerged
code to `dev/ephemeris`. It also does not calculate CIO X/Y/s,
observer velocity, DE441 geometric states, apparent coordinates,
UT1/TT/TDB conversions, source authenticity, Earth-rotation rate
or provide Swiss Ephemeris parity.

## Development and licensing

The new Rust module/test are original code under the existing
**CPAL-1.0 Exhibit A** source header; VirgoStars attribution is
preserved. No new dependencies, kernel files, bundled historical/
current EOP data, GitHub Actions changes, deployment, or release.
Use the authorized **read-only CI** for cargo fmt, tests, Clippy
and Rustdoc on Ubuntu with native tests on Windows and macOS.

**References:** [CelesTrak EOP format and units](https://celestrak.org/SpaceData/EOP-format.php);
[IERS EOP service](https://www.iers.org/iers/en/dataproducts/tools/webservices/webservice-eop);
[IERS EOP data catalog](https://datacenter.iers.org/eop.php).
