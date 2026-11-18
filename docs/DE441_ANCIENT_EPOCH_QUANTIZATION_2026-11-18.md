# DE441 epoch quantization on ancient planetary samples

**Status: exploratory numerical sensitivity, not a physical position error
or MVP accuracy bound.** This follows the [ancient TT–TDB model comparison](SWISS_ANCIENT_TT_TDB_SENSITIVITY_2026-11-18.md).
No normal engine path is changed. Stars and main-server migration remain paused.

## Method

Use 548 TT dates from the two previously recorded Swiss/DE441 planetary
grids, spanning astronomical years −12999 through 3999. At each date obtain
the separately executed geocentric ERFA `dtdb` offset in seconds and form a
split TDB Julian date. The local [`checked_de441_epoch`](../crates/ephemeris-core/src/de441_epoch_bridge.rs)
measures the actual rounding when that split value is collapsed to the
single `f64` TDB Julian date consumed by the classic DE441 reader. The signed
error is rounded minus split, in seconds. Subday arithmetic itself also uses
binary64 and is not arbitrary-precision arithmetic.

For each rounded TDB, select the immediately adjacent representable `f64`
in the direction of the split input; when the error is zero select the next
larger value. Calculate the local apparent geocentric ICRF and experimental
pole-based ecliptic directions at both representable times, for the Sun,
Moon, Mercury, Venus and Mars–Pluto system barycenters. The 5,480 body/date
combinations include 1,000 before year −9998. Check the source grid, its
Swiss flags and original Swiss/DE441 fixed-axis comparisons, as well as the
bridge's rounding and adjacent-step consistency.

The separation between outputs at adjacent times measures **one full
representable step**, not the actual error at the unrepresentable split
instant. Both local outputs use the same numerical implementation and DE441;
the comparison cannot establish the true trajectory, the physical ancient
TT–TDB relation, or correctness of absolute ancient orientation. ERFA is
an extrapolated reference model at remote epochs, not an observation.

## Results

| Grid | Cases | Largest split-to-f64 error | Largest adjacent TDB step | Largest ICRF direction change over one step |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 2,760 | 20.010 µs | 40.234 µs | 0.000025622″ |
| Interleaved | 2,720 | 19.411 µs | 40.234 µs | 0.000025623″ |

The largest ICRF change is the Moon in October of year −10625; its local
ecliptic change is approximately the same. In the earlier-than-Horizons
subset, the largest actual split-to-f64 timing error is about 19.950 µs.
The errors and adjacent steps are properties of the sampled inputs and
representation; the angular changes also include all numerical behavior in
the existing apparent pipeline, including its own internal rounding.

These figures are **not** a physical accuracy claim, a bound on actual
position error, or a guarantee between sampled dates. They do not validate
historical UT1/ΔT, speeds, topocentric events or the full ecliptic convention.
The existing checked bridge can reject excessive caller-specified epoch
rounding, but it does not make DE441 evaluate an unrepresentable split date.

## Reproduction

Build the native Node addon and the Rust diagnostic. Supply the classic
DE441 file, a recorded fixed-axis grid and its SHA-256, a Python executable
with pyerfa 2.0.1.5 and NumPy 2, and a new output path. The audit verifies
source, kernel, interpreter, reference package, projector, addon and helper
hashes; outputs and reference data are not tracked.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_de441_epoch_quantization
node bindings/node/scripts/audit-de441-epoch-quantization.cjs KERNEL KERNEL_SHA SWISS_GRID GRID_SHA PYTHON PROJECTOR NEW_REPORT
```

- DE441 SHA-256: `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
- Baseline input SHA-256: `f055b695016353838a168ab32ca34ea99fcdc385e1e0166d3330ca7450eb2264`.
- Interleaved input SHA-256: `7dd00e18c3a3935446237bd357a23532edd87fe107ffc7ac435dc77436db942c`.
- Baseline output SHA-256: `33ce31444281f1a4d27e2505605cd6b86508883986eff4363a63db59108e5b43`.
- Interleaved output SHA-256: `b758a65ffb682c29848675ce840562503693912e504a76d5cd3709a0797eb63c`.

Both outputs reproduced byte-for-byte on the same Windows build using Python
3.12.14, NumPy 2.3.5, pyerfa 2.0.1.5, ERFA 2.0.1 and SOFA 20231011.
