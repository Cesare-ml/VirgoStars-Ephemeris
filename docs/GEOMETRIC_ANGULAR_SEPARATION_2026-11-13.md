# Geometric angular separation — experimental Rust primitive

Added 2026-11-13. **Status: synthetic mathematical regression only; no
independent NASA/JPL accuracy validation is implied.**

The canonical Rust crate exposes
`geometry::geometric_angular_separation_rad(first, second)`. This is a
small, dependency-free building block for future Sun–Moon elongation and
astronomical event investigations; it is **not** a lunar phase solver and does
not calculate an astrological aspect.

## Contract

- Both inputs are **nonzero**, finite Cartesian displacement vectors from the
  **same observer/center at the same epoch and in the same reference frame**.
  These metadata constraints are the caller's responsibility; arrays have no
  built-in target, center, epoch or frame tags.
- Inputs can use any consistent linear units (e.g., km). Independent positive
  rescaling of the vectors does not change the angle.
- The return value is a geometric unsigned separation in **radians**, in the
  inclusive interval **[0, π]**. Zero denotes alignment, π opposition.
- An all-zero direction returns `AngularSeparationError::UndefinedDirection`;
  NaN/Infinity in either vector returns `NonFiniteInput`.
- No timescale conversion, ephemeris read, center/frame alignment, light-time,
  annual aberration, gravitational deflection, parallax, or refraction occurs.

Example (synthetic vectors in one common frame and from one center):

```rust
use virgostars_ephemeris_core::geometry::geometric_angular_separation_rad;

let radians = geometric_angular_separation_rad(
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
)?;
assert!((radians - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Numerical design

Each input is first scaled by its largest absolute component, not by the
potentially overflowing/underflowing Cartesian distance. The angle is then
`atan2(|a × b|, a · b)`. Unlike `acos(dot / (|a| |b|))`, this
retains resolution for tiny positive angular separations. Scaling also
supports finite subnormal values and values close to `f64::MAX`.

## Validation and integration gates

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Five deterministic integration regressions check cardinal angles,
near-conjunction/opposition, enormous and subnormal magnitudes, symmetry,
positive distance-scale invariance, and explicit invalid-vector errors.
These are **synthetic** tests, not real Sun/Moon numerical comparisons.

For future Moon-facing clients, derive **observer-centered geometric**
Sun/Moon displacement vectors with independently validated DE441/SPK or an
equivalent accepted source and matched epochs/frames. Apply separately
specified corrections before naming any result **apparent**. Lunar
illuminated fraction additionally requires Sun–Moon–observer geometry,
not just observer-centered elongation. Do not generate user-facing lunar
calendar predictions from this primitive alone.

Original code is CPAL-1.0 (see repository `LICENSE` and `NOTICE`).
No external data or dependencies are added. Required public attribution:
**Powered by VirgoStars** — https://www.virgostars.it/.
