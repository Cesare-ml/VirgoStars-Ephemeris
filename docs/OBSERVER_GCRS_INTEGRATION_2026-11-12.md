# Experimental observer GCRS integration — 2026-11-12

**Branch:** `integration/observer-itrs-gcrs-20261112`  
**Status:** draft, non-production, pending independent scientific acceptance.  
**Goal:** integration testing across existing component PRs, not prematurely merging or reinterpreting their claims.

## Exactly what is integrated

This branch starts from the **unmerged** #87 commit
`c27b6c2460c455f42253e174427910232700bf00`
(the standalone matrix composer). It includes unchanged snapshots
of module, test, and documentation blobs from the verified heads:

| PR | Head | Integrated component |
| --- | --- | --- |
| #75 | `9fc954c7b70eb1aebdf3ad0fc65ce72ba42ca107` | IAU 2000 Earth Rotation Angle, UT1, and rate |
| #82 | `af503df5427ca4fc3ea803ff01a5d6a83fc7793d` | WGS84 geodetic → ECEF Cartesian meters |
| #83 | `d5c17ad4a28fa593eac1c24b7799a46d5e3dee3a` | TIRS → CIRS position + spin-only rate |
| #84 | `879fc58faac7c7813445d4e3fbe9ba73f2d73af8` | ITRS ↔ TIRS polar motion and TIO locator |
| #86 | `0779eb35e4a9e6c98adb2c3ff2d71d31903f1bae` | GCRS ↔ CIRS CIP/CIO orientation |
| #87 | `c27b6c2460c455f42253e174427910232700bf00` | ITRS ↔ GCRS matrix composer (branch ancestor) |

The original feature branches and their PRs remain open and unchanged.
Source exports from `lib.rs` are reconciled, without changing the
underlying separately reviewed mathematical components. This
integration does not yet include #78–81 or other apparent-position work.

## New end-to-end Rust API

`observer_gcrs::wgs84_observer_to_gcrs(site, orientation)`
accepts only explicit values and returns:

- `position_itrs_km`: adopted Earth-fixed observer Cartesian position;
- `position_gcrs_km`: same observer rotated into GCRS kilometer axes;
- `era_rad`: actual value calculated from the supplied, **UT1-tagged**
  split Julian date.

`Wgs84ObserverSite` inputs:

- east-positive longitude and north-positive **geodetic** latitude in
  **radians**, with range validation performed by the WGS84 module;
- WGS84 **ellipsoidal** height in **meters** (not height above sea level).

`ObserverOrientationInputs` inputs:

- `ut1_epoch: JulianDate`, with required `TimeScale::Ut1`, from a
  physically obtained UT1 epoch. No implicit UTC↔UT1 conversion.
- `polar_xp_rad`, `polar_yp_rad`, `tio_locator_sp_rad`:
  externally supplied IERS polar motion / TIO locator in **radians**,
  matching the desired date and terrestrial reference realization.
- `cip_x`, `cip_y`: **dimensionless** GCRS Cartesian components
  of the CIP unit pole, not latitude/longitude or angles.
- `cio_locator_s_rad`: externally supplied **radians** for the CIO
  locator s, from a consistent *TT-based* IAU/CIO model.

Every input must refer to the **same physical epoch**. The API verifies
type/finite/range/rotation constraints, but cannot prove the provenance,
timing, reference model or physical accuracy of the supplied EOP/CIP
values. There is no silent zero-EOP assumption or approximate fallback.

The implemented mathematical pipeline is:

```text
WGS84 geodetic (lon, lat, ellipsoidal height)
  → Earth-fixed X/Y/Z meters and kilometers (adopted ITRS realization)
  → W(t) from externally supplied xp, yp, TIO s'
  → ERA(UT1) from strict split Julian date
  → C(t) from externally supplied GCRS CIP X/Y and CIO locator s(TT)
  → compose GCRS→ITRS rotation R = Wᵀ · Rz(-ERA) · C
  → position_GCRS = Rᵀ · position_ITRS
```

The input `ut1_epoch` is the only internally calculated time-dependent
source. All other physical Earth-orientation quantities are
caller-provided. For example, `ut1_epoch` with date-incorrect CIP
values is still mathematically accepted: it is the caller's obligation
to prevent this.

This is a **position-only** transform. It is not an inertial state
vector, a geocentric planetary position, a barycentric JPL/DE441 state,
or a topocentric apparent sky direction. A moving observer's velocity
would require differentiated W, C, ERA and station motion on a coherent
time scale and cannot be obtained by rotating an ECEF velocity alone.

## Independent mathematical fixture

`tests/observer_gcrs.rs` combines *two separately published*
SOFA/ERFA test references:

1. `t_gd2gc`, `ERFA_WGS84`, input longitude `3.1`, latitude
   `-0.5` radians, ellipsoidal height `2500.0` meters,
   with independently published Earth-fixed XYZ in meters.
2. `t_c2tcio`, published GCRS→ITRS `RC2T` matrix, `ERA =
   1.75283325530307`, published `RPOM` and `RC2I` matrices.

The integration uses split UT1
`JulianDate::new(2453737, 0.0, TimeScale::Ut1)`, equivalent to
MJD UT1 `53736.0`, whose ERA matches the published fixture.
The polar angles xp,yp and s′ are those of the independent
`t_pom00`/ `t_c2tcio` fixture. **CIP X,Y** are extracted from
the *published* `RC2I` last row, and **CIO s** is recovered
geometrically from the same published orientation matrix.
Therefore these X/Y/s inputs are **fixture-derived**, not
independently validated physical IAU 2006/2000A time-series values.

The two independent published reference outputs can be combined by
the transpose of published `RC2T` times the published WGS84 XYZ,
giving expected GCRS kilometers:

```text
[  782.6878215238056,
 -5548.791492538574,
 -3041.1420288982213 ]
```

The integration test verifies these values within 2e-8 km (20 µm),
the expected ERA, and the independently published Earth-fixed values.
Other test groups compare the one-call pipeline with all independent
TIRS→CIRS and CIRS→GCRS stages, synthetic zero-parameter geometry,
a rejected TT-labeled epoch, invalid inputs and units/norms.

**This independent composite-fixture test validates rotation order,
frame signs and geodetic units, but does not establish actual physical
UT1/TT/EOP/CIP values for the stated date, time-series accuracy,
Earth-centered vs JPL ICRF alignment, observer velocity, or any
planetary-position parity.** The published RC2T/RPOM/RC2I matrices
are test data only; no SOFA/ERFA implementation code is copied.

## Release and scientific gates remaining

- Native full-workspace validation must pass on Ubuntu, Windows, macOS:
  `cargo fmt --all -- --check`,
  `cargo test --workspace --locked` (debug and release as configured),
  `cargo clippy --workspace --all-targets --locked -- -D warnings`,
  `cargo doc --workspace --no-deps --locked`.
- Supply and independently check real TT-based IAU 2000A/2006 X,Y,s
  time series, UT1 from measured EOP, xp/yp/s′, station reference epoch
  and polar/station velocities.
- Validate an *independent dated* observer GCRS position against a
  trusted SOFA/ERFA/IERS test trace with declared TT, UT1, EOP values
  and observer realization, not merely composition of input matrices.
- Integrate a real DE441 Earth/planet origin and explicit frame,
  time-scale, light-time, aberration and topocentric-parallax pipeline
  with independent JPL/Horizons checks before Swiss compatibility claims.
- Preserve documented CPAL-1.0 source notices, VirgoStars project
  attribution and free/open-source architecture.

No production use, package publication, release, protected-main merge,
secret handling, EOP/network download, cross-repo change, or
unapproved automatic deployment is included.

**Scientific sources:** [IERS Conventions](https://iers-conventions.obspm.fr/conventions_material.php);
[ERFA published numerical tests `t_gd2gc` and `t_c2tcio`](https://github.com/liberfa/erfa/blob/master/src/t_erfa_c.c).
