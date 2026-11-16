# Experimental fixed-axis aberrated six-field pipeline

**Status: local experimental integration, 2026-11-16.** This composes existing
work from `origin/feat/mvp1-aberrated-fixed-spherical-20261111` (`c13640e`)
and the shared six-field unit adapter from the MVP integration branch. It is
not an apparent-of-date or Swiss-compatible result.

## Pipeline and contract

`geocentric_aberrated_fixed_six_fields` takes an externally supplied classic
DE441 reader, target, reception Julian date in **TDB**, a constant passive
X-axis tilt in radians, and an explicit Earth-acceleration finite-difference
step in TDB days. It composes reception light-time, observer-velocity annual
aberration and its reception-time derivative, fixed-axis spherical projection,
and unit conversion. Earth is the center; the target remains the exact `Body`
variant, including planetary-system barycenters.

The result carries longitude and latitude in **degrees**, distance in **AU**,
angular rates in **degrees per reception TDB day**, and radial rate in **AU
per reception TDB day**. It retains reception and emission epochs, light time,
light-time rate, constant tilt, and acceleration provenance including the
**actual representable** Earth acceleration step for the sampled method. The
`geocentric_aberrated_fixed_six_fields_analytic` variant uses the selected
DE441 polynomial's analytic second derivative with no sampling step. The exact
IAU-defined AU is 149,597,870.7 km. The separate
`spherical_from_aberrated_state` projects an existing intermediate without
applying light-time or aberration twice; malformed timing or acceleration-step
metadata are rejected. Conversion rejects nonfinite values, out-of-range
angles, nonpositive distance and unrepresentable outputs.

The axes are ICRF for zero tilt or a caller-specified **fixed** rotation.
There is no moving ecliptic-of-date frame, frame derivative, nutation,
deflection, refraction, topocentric observer or UT1/TT conversion. In
particular, the result is not a complete apparent ecliptic longitude or
`swe_calc_ut` output. The two explicit acceleration variants are numerical
centered velocity differences and the analytic second derivative of the
selected Chebyshev polynomial; neither is an independent physical accuracy
certificate. The branch's optimized batch variant remains separate work.

## Verification and limitations

- The branch's synthetic tests cover all six fields, transverse Lorentz
  aberration and its derivative, fixed tilt, retrograde sign, wrap and pole
  errors, metadata and unit conversion. A synthetic DE441 end-to-end test
  exercises the explicitly stepped composition.
- An optional local test on the existing DE441 file compares the three
  spherical rates with centered differences of this same implementation's
  positions for Mercury, Moon and Mars-system barycenter at TDB JD 2461354.5.
  The interval is ±600 seconds; bounds are `1e-4` degree/TDB-day for angular
  rates and `1e-6` AU/TDB-day for radial rate. This is internal kinematic
  consistency, **not an independent astronomy reference**.
- Independent apparent-position/rate references, broad epochs, station and
  frame/correction parity remain required before any production integration.

```powershell
cargo test --locked --test aberrated_spherical --test aberration_state --test six_field_units
$env:DE441_PATH = '<path-to-classic-DE441>'
cargo test --locked --test aberrated_spherical -- --ignored
```

**CPAL-1.0 modification record (2026-11-16):** reconciled the existing
projection and unit adapter with the current finite-difference acceleration
API, validated acceleration metadata, added composition and real-file
consistency tests, and clarified that the generic projector preserves the
input correction semantics. No third-party source or data added.
