# DE441 header validation regression — 2026-11-10

## Change

Adds `crates/ephemeris-core/tests/de441_header_validation.rs` with deterministic,
in-memory classic DE441 header fixtures. Cases cover valid coverage metadata,
non-finite/negative scalar fields, fractional record counts, invalid pointer
shapes, short headers, and exact file-size checks.

The fixture is deliberately synthetic and contains no real NASA/JPL ephemeris
coefficients. It verifies parsing and rejection behavior, **not** scientific
accuracy. No SPK support, time-scale conversion or new coordinate conventions
are claimed.

## Scientific contract

Unchanged: dates are Julian Date TDB; states are geometric equatorial ICRF
position in km and velocity in km/day. No algorithms or data sources changed.

## Validation

Run on a machine with Rust:
```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The originating agent environment does not provide Cargo; CI must verify
these checks before integration. The independent NASA/JPL validation gate
remains open.

## License / provenance

The tests are original, contain no copied Swiss Ephemeris code and include
the repository CPAL-1.0 Exhibit A notice. The existing license and VirgoStars
attribution remain unchanged.
