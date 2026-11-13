# Geometric lunar illuminated fraction — experimental Rust adapter

Added 2026-11-13. **Not validated against independent NASA/JPL vectors.**
This additive Rust adapter depends on the low-level geometric separation
primitive added by draft PR #167. Do not treat it as a validated Moon-phase
calendar, apparent lunar illumination, or integration into VirgoStars production.

## API and scientific contract

`lunar_phase::geometric_lunar_illumination(&sun, &moon)` accepts two
`de441::StateVector` values. Their required metadata is checked:

- `sun.target = Body::Sun`, `sun.center = Body::Earth`.
- `moon.target = Body::Moon`, `moon.center = Body::Earth`.
- Both `epoch` values must be finite and **exactly equal**, expressed as
  **Julian Date TDB** (no UTC, TT or time conversion).
- Both geometric **Earth-centered ICRF Cartesian positions in kilometers**
  must be finite. Existing `StateVector` velocity fields, in km/day,
  are intentionally ignored.
- Physical Sun/Moon geometry, not a zodiacal angle: the phase angle at
  the Moon is between **Moon→Sun** (`sun.position - moon.position`) and
  **Moon→Earth** (`-moon.position`).
- Result: phase angle in **radians [0, π]** and ideal spherical illuminated
  fraction in **[0, 1]**, with **0 = dark/new-side** and **1 = full-side**.
  This uses `cos²(alpha/2)`, with a complementary half-angle for angles
  above π/2 to avoid spurious nonzero illumination at exact opposition.

Invalid target/center, mismatched epoch, nonfinite inputs, infinite
differences and zero Moon→Sun/Moon→Earth vectors produce explicit errors.
There is **no silent fallback** to the other model or time scale.

This is geocentric *geometric* illumination. No light-time, aberration,
gravitational deflection, topocentric parallax, lunar libration, terrain,
shadowing/eclipses, limb effects or atmospheric optics. The unsigned fraction
cannot tell waxing from waning or determine exact new/full Moon *event times*.
It is not interchangeable with Earth-observed Sun–Moon elongation.

## Caller-supplied classic DE441 example

```rust
use virgostars_ephemeris_core::de441::{Body, De441, JulianDateTdb};
use virgostars_ephemeris_core::lunar_phase::geometric_lunar_illumination;

let mut eph = De441::open("/path/to/de441.eph")?;
let epoch = JulianDateTdb(2_451_545.0);
let sun = eph.state(Body::Sun, Body::Earth, epoch)?;
let moon = eph.state(Body::Moon, Body::Earth, epoch)?;
let result = geometric_lunar_illumination(&sun, &moon)?;
println!("geometric illuminated fraction: {}", result.illuminated_fraction);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The example requires a separately obtained classic DE441 binary file. No
ephemeris file is committed and no test downloads any kernel. DE441 positions
themselves still await documented, independent NASA/JPL validation.

## Direct classic DE441 bridge (experimental)

The additive Rust API
`lunar_phase::geometric_lunar_illumination_from_de441(&mut eph, epoch)`
retrieves **Sun relative to Earth** and **Moon relative to Earth** at the
**same Julian Date TDB** from one caller-supplied, seekable classic
`De441<R: Read + Seek>` reader. The existing reader provides geometric ICRF
positions in km, and preserves the Moon's directly encoded geocentric series
rather than subtracting large absolute barycentric Earth/Moon positions.
The bridge delegates to `geometric_lunar_illumination` instead of
reimplementing the geometry.

```rust
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};
use virgostars_ephemeris_core::lunar_phase::geometric_lunar_illumination_from_de441;

let mut eph = De441::open("/path/to/de441.eph")?;
let result = geometric_lunar_illumination_from_de441(
    &mut eph,
    JulianDateTdb(2_451_545.0),
)?;
println!("fraction: {:.9}", result.illuminated_fraction);
# Ok::<(), Box<dyn std::error::Error>>(())
```

`LunarDe441Error` distinguishes a nonfinite input epoch from the original
`De441Error` (coverage, missing series, file corruption or I/O) and from
the geometric `LunarIlluminationError`. It does not download data,
silently substitute an analytic ephemeris, or convert UTC/TT into TDB.

A new **offline synthetic classic-binary integration fixture** includes
two 32-day records with deliberately chosen geocentric Sun/Moon triangles:
quarter geometry in record one and dark/new-side geometry in record two.
Five deterministic tests cover inclusive coverage and the exact record
boundary; nonfinite epochs; missing Sun, Moon, or Earth–Moon barycenter
series; malformed Chebyshev coefficient and zero lunar vector; injected
read failures and cache reuse after recovery. A passing test proves only
correct reader-to-geometry wiring with synthetic coefficients; it is not
an independently sourced NASA/JPL reference or precision certification.

## Geocentric elongation and conditional waxing/waning longitude halves (2026-11-14)

New `lunar_phase::geometric_lunar_orientation(&sun, &moon, ecliptic_north_icrf)`
builds on the previous, checked Sun/Earth and Moon/Earth `StateVector`
contract. It returns a `GeometricLunarOrientation` with **three distinct
outputs**:

- `illumination.phase_angle_rad` (Moon-vertex angle; radians) and
  `illumination.illuminated_fraction` (ideal spherical fraction, 0–1).
- `geocentric_elongation_rad` (unsigned **Sun–Earth–Moon** 3-D angle,
  radians 0–π). The value generally differs from both the phase angle at
  the Moon and the difference in projected ecliptic longitudes.
- `signed_ecliptic_longitude_difference_rad` (Moon ecliptic longitude
  minus Sun ecliptic longitude, projected onto the plane normal to the
  **caller-supplied** pole, radians in [−π, +π]) and
  `longitude_half: LunarEclipticHalf`.

For conventional astronomical **waxing or waning longitude-half**
interpretation, the caller **must** pass a correctly determined *north
ecliptic pole expressed in ICRF Cartesian axes*, with its explicitly
chosen epoch and ecliptic convention matching the states. **The API does
not compute the physical ecliptic pole or rotate ICRF states into the
ecliptic of date.** A vector such as `[0, 0, 1]` is the ICRF equatorial
north axis, **not** automatically ecliptic north in real astronomy.
The synthetic tests deliberately use an artificial plane with that normal;
they do not constitute a real physical ecliptic solution.

The signed difference is measured using the normal's right-hand
orientation on **projected** Sun/Moon directions:

- Moon longitude ahead of Sun (positive 0–π): `WaxingLongitudeHalf`.
- Moon longitude behind Sun (negative −π–0): `WaningLongitudeHalf`.
- Exactly projected aligned/opposed (0 or π): `ConjunctionOrOpposition`.
- A zero/nonfinite reference pole, zero Sun/Moon projected longitude or
  invalid state metadata fails explicitly. Both 3-D directions retain
  overflow-safe numeric scaling before projections and products.

These half-plane labels are **not measurements of whether the illuminated
fraction is currently increasing/decreasing with time**. They are
geometrical longitude-sector labels subject to the supplied pole and
its provenance. Near conjunction/opposition the sign can be sensitive;
no event-time solver or empirical rate is involved.

The matching direct reader convenience API is
`geometric_lunar_orientation_from_de441(&mut eph, epoch_tdb, ecliptic_north_icrf)`.
Both Sun/Earth and Moon/Earth states come from the **same** classic DE441
reader, using the same already-TDB Julian date and the original canonical
Rust interpolation, not a separate algorithm. Both reader adapters share
a single internal `read_sun_moon_de441` helper and existing cache.

```rust
use virgostars_ephemeris_core::de441::{De441, JulianDateTdb};
use virgostars_ephemeris_core::lunar_phase::geometric_lunar_orientation_from_de441;

let mut eph = De441::open("/path/to/de441.eph")?;
// Example only: caller must supply its independently validated ICRF
// ecliptic-north vector for this date and ecliptic convention.
let validated_ecliptic_north_icrf = [0.0, 0.0, 1.0]; // Placeholder, NOT physical ecliptic north.
let snapshot = geometric_lunar_orientation_from_de441(
    &mut eph,
    JulianDateTdb(2_451_545.0),
    validated_ecliptic_north_icrf,
)?;
println!("3-D elongation: {} rad", snapshot.geocentric_elongation_rad);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Seven additional synthetic direct-state regressions cover physically distinct
triangle angles, both longitude half-planes, exact conjunction/opposition,
nonzero latitude, independent pole scaling/sign reversal, near-opposition
signed continuity, invalid poles, undefined projections and matching
target/center/epoch errors. A further synthetic binary bridge test checks
the second 32-day record, geocentric elongation versus fraction, invalid
pole and outside-coverage propagation. No external data was downloaded.
All results remain **geometric and experimental** pending independently
traceable JPL/NAIF vectors, accuracy budgets and physical frame conventions.

## Test evidence and limitations

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Six direct-vector synthetic Rust regression tests exercise:
- Exactly new/full/quarter geometric triangles (including the difference
  between **45° geocentric elongation** and **90° lunar phase angle**).
- Tiny positive near-new illuminated fraction.
- Incorrect target/center and mismatched TDB epochs.
- Nonfinite epochs/components and coincident/zero position vectors.
- Explicit rejection of floating-point subtraction overflow.
- Invariance to velocity values, which the primitive does not consume.

Passing these tests and the bridge tests establishes a mathematical/API contract, **not**
scientific ephemeris parity. Before downstream use, compare full real-kernel
Sun/Moon states and illuminated fractions against independently documented
reference cases and tolerances. Do not ship user-facing Moon predictions
until those validation gates and correction-mode decisions are approved.

Original source is CPAL-1.0; see `LICENSE` and `NOTICE`.
Third-party data remains separately licensed. Attribution:
**Powered by VirgoStars** — https://www.virgostars.it/.
