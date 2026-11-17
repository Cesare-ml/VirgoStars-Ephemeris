# Swiss catalog inventory and long-range planetary diagnostics — 2026-11-17

**Status: experimental, local only; neither catalog parity nor apparent
planetary coverage for years 0–5000 is established.** The owner requires all
stars covered by Swiss Ephemeris before the main server can use this engine.

## Inventory and first integration

`scripts/inventory_swiss_stars.mjs` enumerates every uncommented record of the
caller-supplied `sefstars.txt`, preserving line numbers, traditional names and
designations, including unnamed records and repetitions. Comments are removed
before splitting fields. It fails on malformed input rather than skipping rows.
The local file `database/effemeridi/astrodienst/sefstars.txt` in Virgo Stars has
SHA-256 `e0d8fafc6ea866533cecc2aa0e4ce73094d5c79697da5fe45478c73485cf1b91`.
The 2018 snapshot contains 1,350 records, 763 named records, 729 distinct
case-folded names and 1,113 distinct case-folded designations. There are 33
repeated names and 186 repeated designations. These are lookup inventory
counts, **not counts of distinct physical stars**: records include duplicates,
aliases, unnamed entries, test records and nonstellar objects. Other Swiss
versions or built-in catalogs are not inventoried by this observation.

The existing IAU-CSN 2022 and HIP2 path found 315 exact-name IAU leads among
the 729 names; 414 had no exact name match. Of the 314 with an IAU HIP ID,
one was absent from the supplied HIP2 file and 195 passed the existing strict
numerical screen. `audit-star-quality.mjs` now isolates missing HIP records;
unrelated catalog errors still abort. The existing manifest builder generated a
195-entry **review-required candidate** manifest, including all 40 former
entries. No Swiss astrometric values were copied. The caller-provided IAU and
HIP2 values, their source hashes, and the zero radial-velocity assumptions
are retained by that manifest.

Exact spelling is not an identity validation: in this snapshot six names in
the 195-entry manifest occur with multiple designations (Algedi, Proxima
Centauri, Menkar, Ukdah, Tabit and Alnasl). Each needs component and lookup
behavior review, including Menkar which was already in the 40-entry candidate.
The 195 stars have been calculated in a single modern-date wiring check; this
does not validate individual identities, velocities, long-term stellar motions
or Swiss numerical parity. Unnamed designations and the remaining names are
not covered by the manifest merely because they were inventoried.

The external source, inventory, name list, candidate and quality reports and
manifest are not committed. Local generated artifacts are under ignored
`local-data/swiss-census-2026-11-17/`. Regenerate them in a new empty output
directory with the three tools in order:

```text
node scripts/inventory_swiss_stars.mjs <sefstars.txt> <inventory.json> <names.json>
node scripts/iau_star_candidates.mjs <IAU-CSN.txt> <names.json> <https-source-url> <candidates.json>
node bindings/node/scripts/audit-star-quality.mjs <candidates.json> <hip2.dat> <quality.json>
node scripts/build_mvp_star_manifest.mjs <candidates.json> <quality.json> <IAU-CSN.txt> <hip2.dat> <manifest.json>
```

The separate experimental Virgo Stars branch loads this manifest and exercises
all 195 in loopback batch and natal flows. Its batch limit is now 2,000, still
an explicit limit; this is not a production activation or a performance
guarantee at that limit.

## Geometric DE441 layer over 0–5000

The Node binding now exposes `Engine.geometricStateTdb(jdTdb, body)` for Sun,
Moon, Mercury, Venus and the Mars–Pluto **system barycenters**, relative to
Earth. It delegates to the existing Rust DE441 state reader and returns ICRF
position in km and velocity in km/TDB day. It applies no light-time,
aberration, precession, nutation, Delta T or civil-time conversion. It is a
diagnostic geometric state, not a tropical longitude or an apparent position.

The local file was compared with [JPL Horizons](https://ssd-api.jpl.nasa.gov/doc/horizons.html)
on July 1 at 12:00 TDB of astronomical Gregorian years 0, 1000, 2000, 3000,
4000 and 5000, 10 targets × 6 epochs. Queries explicitly requested geometric
ICRF vectors, Earth center, TDB and KM-D, and their responses identify both
target and Earth source as DE441. The maximum absolute component differences
were `9.5367431640625e-7 km` in position and `1.3969838619232178e-9 km/day` in
velocity, below preselected diagnostic limits of `0.001 km` and
`0.001 km/day`. The complete queries, responses and per-body results are in
the ignored local `horizons-geometric-0-5000-verified.json`; the supplied
kernel SHA-256 is `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`.
Run the explicit manual network diagnostic with:

```text
node bindings/node/scripts/compare-horizons-geometric-range.cjs <de441.eph> <sha256> <new-output.json>
```

This is an independent implementation comparison using the **same DE441
solution**, over 60 sampled vectors. It checks interpolation and center/frame
handling on those samples; it does not independently validate DE441 itself,
all dates, apparent positions, UT, astrological quantities or the model's
physical accuracy in the distant past or future.

## Remaining work

1. Resolve designation-only records, duplicate names, multiple systems,
   historical aliases and nonstellar objects individually; compare Swiss lookup
   semantics, not just case-folded exact spelling. Review all catalog candidates.
2. Implement and validate a suitable long-term precession/ecliptic and nutation
   model. The [IAU SOFA report](https://www.iau.org/static/science/scientific_bodies/working_groups/191/annual_report_wg191.pdf)
   identifies the Vondrak et al. long-term precession model as a candidate;
   its integration and independent validation have not been done here.
3. Add explicit Delta T estimation policies with uncertainty for historical
   and future UT, calendar/year-zero handling through the backend, and
   validation of apparent planetary longitude/rates over the full interval.
   Do not extend the existing 1900–2100 orientation guard without a model and
   evidence. Chiron requires a separate appropriately covered SPK.
