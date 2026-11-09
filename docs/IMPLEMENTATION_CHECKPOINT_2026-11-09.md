# Implementation checkpoint — 9 November 2026

Status: **IMPLEMENTED_UNVERIFIED** scientifically; not production-ready or released.

## Completed in branch PR #1

The first independent runtime implementation is written in Node.js ESM (Node >=20), with no third-party dependencies or Swiss Ephemeris imports. The SPK/DAF reader inspects binary records and descriptors in both endiannesses, evaluates type-2 Chebyshev coefficients and their time derivatives, and composes J2000 geometric positions and velocities along target/center chains.

An experimental file-free mode provides JPL *approximate planetary element* Table 1 heliocentric vectors for Mercury, Venus, Earth–Moon barycenter, Mars, Jupiter, Saturn, Uranus and Neptune (1800–2050), plus a separate USNO approximate apparent Sun model (1800–2200). These models are **not equivalent to Moshier or Swiss**; no Moon, Earth-center correction, Pluto, precession/nutation, light-time or apparent corrections for the JPL planetary elements are provided.

## Actual executed tests

In a local isolated container (Node 22.16.0), **9/9 tests passed**, covering synthetic DAF little-/big-endian metadata, Chebyshev values and velocities, a synthetic Sun–Earth–EMB chain, segment endpoints, malformed/out-of-range cases and basic analytical sanity. The committed SPK/source and test blobs were compared against the locally executed blobs by SHA; there is no GitHub Actions setup.

**This is not real JPL kernel validation.** No authentic DE440s file or JPL Horizons reference result could be downloaded/tested in the available isolated runtime. Real scientific accuracy remains unknown.

## Running the prototype

With Node >=20, use npm test. For manually downloaded kernels, run:

node scripts/inspect-spk.mjs /path/to/de440s.bsp

For a geometric relative state with verified TDB seconds from J2000 (NOT UTC):

node scripts/inspect-spk.mjs /path/to/de440s.bsp 0 10 399

This utility reads the whole file into memory; start with modern compact SPKs and avoid multi-GB kernels. Output lists segment metadata, SHA-256 and any geometric J2000 state.

## Next release gates

1. Pin a real official NASA/NAIF SPK filename, URL, hash and reference configuration.
2. Validate segment selection, SPK type-2 interpolation and velocity against NAIF/Horizons at multiple epochs.
3. Implement real TDB/TT/UTC model with leap seconds; then time-dependent apparent/ecliptic-of-date coordinate transformations and observer corrections.
4. Define per-feature tolerances, numerical provenance and license audits before exposing public stable API.
5. Functions package remains separately staged and production VirgoStars/Astrolabio remain unchanged.

## Further implementation checkpoint — 9 November 2026

Additional original source and tests on PR #1:
- `src/spk3.mjs`: SPK type-3 Chebyshev series for position and independently stored velocity; type 2 remains supported. The geometric state resolver now dispatches based on segment type.
- `src/time.mjs`: explicit modern-era UTC -> TAI -> TT -> **approximate** TDB conversion, IERS Bulletin C 72 / NIST leap-second offsets (1972 through 2026), rejecting invalid dates, `:60` leap-second instants, out-of-range dates and implicit timezone strings.
- `approximateGeometricStateFromUtc()`: optional adapter with full warning and time-model metadata, not a high-precision final time conversion.
- `test/type3-time.test.mjs`: six additional tests on independent synthetic type-3 segments and time-scale boundary cases.

**Actually executed in isolated Node 22 runtime:** Ephemeris `npm test`: **15 passed, 0 failed**. Functions remains 4/4 locally. Code has NOT been tested with an authentic NASA/JPL SPK: fetching `de440s.bsp` from the isolated environment failed. The precision of TDB approximation and SPK parsing requires an independent real-kernel/NAIF comparison.

IERS Bulletin C 72, 6 July 2026, confirms TAI-UTC=37s effective from 2017 and NO leap second at end of December 2026: https://datacenter.iers.org/data/html/bulletinc-072.html . Earlier leap transitions: https://www.nist.gov/pml/time-and-frequency-division/time-realization/leap-seconds . Leap tables **must be refreshed** for dates starting 2027, and the prototype deliberately rejects those dates rather than silently applying a stale offset.

The first release must still add authoritative leap second instant handling, robust TDB model, all required IAU frame conversions, independent tolerance measurements, non-Type-2/3 support as needed and real-kernel validation.
