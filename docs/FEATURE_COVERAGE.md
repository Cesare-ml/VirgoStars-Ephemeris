# Functional coverage target — 2026-11-10

The project goal is to cover the documented capabilities of Swiss Ephemeris,
including its astronomical and astrological functions. This is a development
target, not a claim that they are already implemented, API-compatible or
numerically equivalent. Use the [Swiss programming documentation](https://www.astro.com/swisseph/swephprg.htm)
as a behavioral inventory; implement independently and document conventions,
data provenance, supported ranges, missing cases and errors. This preliminary
family-level inventory must be expanded into individual function/flag/variant
acceptance cases before a feature can be considered complete.

| Capability family | Intended owner | Current Rust development status |
|---|---|---|
| Planet, Sun and Moon positions, speeds, barycentric/heliocentric/geocentric and topocentric variants | Ephemeris core | Classic external DE441 geometric ICRF vectors; fixed-axis spherical projection. Apparent and topocentric positions remain open. |
| Asteroids, minor bodies, planetary satellites, hypothetical objects and lunar nodes/apsides | Ephemeris core, with astrology-specific conventions in functions | Not yet supported by the core's classic planet reader. Additional data and models require provenance and coverage decisions. |
| Fixed stars, names, catalog lookup, magnitude and motion | Ephemeris core | Not implemented; source catalogs and rights require review. |
| Calendars, Julian days, UTC/TAI/TT/TDB/UT1, ΔT and sidereal time | Ephemeris core | Limited typed TDB/J2000 helpers; civil time, leap seconds, Earth orientation and historical ΔT remain open. |
| Frames, precession/nutation, coordinate transforms, observer geometry, apparent corrections and refraction | Ephemeris core | Fixed-axis geometric spherical projection only; date-dependent rotations and their derivatives are open. |
| Solar/lunar eclipses, occultations, rise/set, meridian passages, crossings and planetary phenomena | Ephemeris core with functions-layer orchestration where appropriate | Not implemented in the Rust core. |
| Heliacal events and visibility, atmospheric and optical models | Ephemeris core / functions | Not implemented; explicit observer and environmental inputs required. |
| Tropical/sidereal zodiacs, ayanamshas, houses, house positions and Gauquelin sectors | Astrological Functions, backed by the Rust core | Not implemented in the inspected functions repository. |
| Names, configuration, units, compatibility flags, lifecycle and API adapters | Core and language bindings | No stable Swiss-compatible facade or released bindings. |

Priority sequence: (1) usable DE441 states, time scales and geometric
projections; (2) date frames and apparent/topocentric positions; (3) stars,
minor bodies and the astrological functions; (4) event searches and visibility;
(5) exhaustive compatibility and independently reproduced reference cases.
These phases overlap where dependencies permit. A clean-room Rust engine and
thin language bindings remain the architecture; public access to data does
not grant redistribution rights. House and zodiac algorithms belong in the
functions layer unless an explicit later architecture decision changes that.

The current local JPL file is a classic DE441 binary; the local `.se1` files
are a separate Astrodienst format and are not read by this Rust implementation.
Dataset coverage is not equivalent to completed calculation functionality.
