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
| Planet, Sun and Moon positions, speeds, barycentric/heliocentric/geocentric and topocentric variants | Ephemeris core | Classic external DE441 geometric ICRF vectors and an experimental time/location topocentric geometric CLI. Apparent positions, date-frame speeds and planet-center support remain open. |
| Asteroids, minor bodies, planetary satellites, hypothetical objects and lunar nodes/apsides | Ephemeris core, with astrology-specific conventions in functions | Not yet supported by the core's classic planet reader. Additional data and models require provenance and coverage decisions. |
| Fixed stars, names, catalog lookup, magnitude and motion | Ephemeris core | Not implemented; source catalogs and rights require review. |
| Calendars, Julian days, UTC/TAI/TT/TDB/UT1, ΔT and sidereal time | Ephemeris core | Proleptic calendar ↔ split JD; TAI↔TT and a limited UTC-offset/TT/TDB route for the geometric CLI. Complete UTC/leap-second handling, Earth orientation and historical ΔT remain open. |
| Frames, precession/nutation, coordinate transforms, observer geometry, apparent corrections and refraction | Ephemeris core | Fixed-axis projection and IAU 2006 mean-of-date positions with WGS84 observer in the experimental CLI; nutation, apparent corrections and date-frame derivatives are open. |
| Solar/lunar eclipses, occultations, rise/set, meridian passages, crossings and planetary phenomena | Ephemeris core with functions-layer orchestration where appropriate | Not implemented in the Rust core. |
| Heliacal events and visibility, atmospheric and optical models | Ephemeris core / functions | Not implemented; explicit observer and environmental inputs required. |
| Tropical/sidereal zodiacs, ayanamshas, houses, house positions and Gauquelin sectors | Astrological Functions, backed by the Rust core | Not implemented in the inspected functions repository. |
| Names, configuration, units, compatibility flags, lifecycle and API adapters | Core and language bindings | No stable Swiss-compatible facade or released bindings. |

**Owner-approved product order (2026-11-11):** [MVP 1 essential calculations + stars/houses/loader, MVP 2 all remaining used calls, MVP 3 all remaining Swiss APIs](THREE_MVP_SWISS_REPLACEMENT_2026-11-11.md). The broader coverage table above remains a functional inventory, not a release-status or MVP priority checklist.

Backend-first priorities, concrete call sites and acceptance cases are tracked
in [the compatibility inventory](BACKEND_SWISS_COMPATIBILITY_2026-11-10.md)
and [work packages](BACKEND_WORK_PACKAGES_2026-11-10.md). Under the owner-approved MVP order,
prioritize **Sun/Moon/physical planets and speeds**, Julian-day forward/reverse
conversion, five used house systems, fixed stars and the Node load/compatibility
interface **in MVP 1** (scaffolding interfaces in parallel). MVP 2 covers
remaining currently used Swiss functionality, including nodes/apogees/Chiron,
horizon/rise/transit and eclipse workflows. MVP 3 covers all other documented
Swiss Ephemeris capabilities. Independent reference work starts alongside implementation.
This ordering does not remove the long-term full-coverage target.
A clean-room Rust engine and
thin language bindings remain the architecture; public access to data does
not grant redistribution rights. House and zodiac algorithms belong in the
functions layer unless an explicit later architecture decision changes that.

The current local JPL file is a classic DE441 binary; the local `.se1` files
are a separate Astrodienst format and are not read by this Rust implementation.
Dataset coverage is not equivalent to completed calculation functionality.
