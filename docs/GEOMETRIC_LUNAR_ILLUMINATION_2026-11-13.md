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

## Test evidence and limitations

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Six synthetic Rust regression tests exercise:
- Exactly new/full/quarter geometric triangles (including the difference
  between **45° geocentric elongation** and **90° lunar phase angle**).
- Tiny positive near-new illuminated fraction.
- Incorrect target/center and mismatched TDB epochs.
- Nonfinite epochs/components and coincident/zero position vectors.
- Explicit rejection of floating-point subtraction overflow.
- Invariance to velocity values, which the primitive does not consume.

Passing these tests establishes a basic mathematical contract, **not**
scientific ephemeris parity. Before downstream use, compare full real-kernel
Sun/Moon states and illuminated fractions against independently documented
reference cases and tolerances. Do not ship user-facing Moon predictions
until those validation gates and correction-mode decisions are approved.

Original source is CPAL-1.0; see `LICENSE` and `NOTICE`.
Third-party data remains separately licensed. Attribution:
**Powered by VirgoStars** — https://www.virgostars.it/.
