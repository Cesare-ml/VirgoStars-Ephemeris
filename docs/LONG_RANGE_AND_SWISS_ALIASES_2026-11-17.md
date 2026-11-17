# Long-range apparent experiment and catalog expansion — 2026-11-17

**Status: experimental/local; not a validated 0–5000 production ephemeris.**
The existing `calcTdb`/`calcUt` guard and the requirement to cover all Swiss
stars before using the engine on the main server remain unchanged.

## Catalog

`attach_swiss_alias_candidates.mjs` adds candidate names and comma-prefixed
designations only where a selected name has one Swiss designation and every
row in that designation group has identical frame, coordinates, motion and
magnitude fields. Ambiguous names and conflicting rows are not merged. The
manifest binds the external Swiss snapshot by SHA-256; no Swiss astrometry is
imported into the engine. These are lookup **candidates**, not independently
established stellar identities. The seven withheld canonical names are
Algedi, Proxima Centauri, Menkar, Ukdah, Tabit, Alnasl and Alcor.

`extend_star_manifest_nec.mjs` screens exact Swiss traditional names against a
caller-provided IAU WGSN Naked Eye Catalog (NEC) snapshot, requires a unique
HIP, finite NEC V magnitude, ICRS coordinates within 30 arcseconds and the
existing strict HIP2 quality filter. It prevents duplicate names/IDs and binds
all inputs by hash. The locally supplied NEC snapshot has **unverified file
provenance**; the [WGSN describes the NEC](https://exopla.net/star-names/database/naked-eye-catalog/)
as including multiple components and integrated objects. A numerical match is
not an individual identity or magnitude review.

In the local snapshot, 102 names satisfied the preliminary filters, one HIP
was absent from HIP2, and one new strict candidate was added: Zhou, HIP 77233,
whose two input coordinates differ by about 0.048 arcsecond. The final
experimental manifest has **196 stars and 277 candidate aliases**, including
comma-prefixed designations. Unurgunite was not automatically added: a same-
name NEC row and the Swiss position differ by roughly 21 arcminutes, despite
a HIP lead, and need investigation. The 196-star manifest and detailed reports
remain in ignored `local-data/swiss-census-2026-11-17/`; the local HTTP star,
batch and natal flows passed with this manifest and all 277 aliases resolved
to their selected entries in a separate engine test. These tests do not prove
the selected entries correspond to the intended physical objects.

```text
node scripts/attach_swiss_alias_candidates.mjs <manifest.json> <sefstars.txt> <new-manifest.json> <new-report.json>
node scripts/extend_star_manifest_nec.mjs <manifest.json> <sefstars.txt> <nec.csv> <hip2.dat> <new-manifest.json> <new-report.json>
```

After the NEC extension, rerun the alias tool on its new manifest to include
safe candidate designations of newly selected stars. Input/outputs must be
distinct; output files are not overwritten. No complete Swiss compatibility,
lookup ordering, component identity or distribution decision is asserted.

## Experimental apparent direction, TDB

`long_term_precession` implements the long-term Vondrak et al. mean equator and
ecliptic poles from the published coefficient tables; the result uses J2000
mean axes. The experimental apparent path applies the existing IAU 2006 frame
bias, the long-term precession/ecliptic, and the existing 77-term nutation
coefficients with full IERS polynomial arguments extrapolated to the selected
epoch. This combination is **not standard IAU 2000B** and its physical accuracy
outside the modern interval has not been established. Light time, solar
deflection and annual aberration are applied; no UT or Delta T conversion is
performed. TT is derived approximately from TDB. Rates include a centered
numerical matrix derivative. The experiment is limited to Julian epochs
0.0–5001.0, and rejects epochs outside that interval.

Explicit diagnostic Node methods:

- `experimentalLongTermMeanEclipticTdb(jdTdb, body)`: corrected direction in
  the **mean** ecliptic of date; nutation omitted.
- `experimentalLongTermApparentTdb(jdTdb, body)`: candidate true-ecliptic
  direction with extrapolated nutation; notes report the approximation.

Both return six fields in degrees/AU and per TDB day, for Sun, Moon, Mercury,
Venus and Mars–Pluto **system barycenters**, geocentric. Neither method is a
Swiss-compatible facade, production activation, long-range UT policy, house
calculation or small-body/Chiron solution. The standard engine methods retain
their previous range checks.

## Local independent implementation checks

- ERFA 2.0.1.5 `ltp` and `ltpecl` at six epochs from 0–5000: maximum matrix/
  pole component difference `1.6653345369377348e-16` (test limit `2e-14`).
  This compares independent implementations of the same Vondrak model, not
  the physical truth of the model.
- ERFA 2000A longitude nutation at 101 Julian epochs, 50 years apart: maximum
  difference `0.0022332554169075536 arcsecond` (predeclared diagnostic limit
  `0.1 arcsecond`). Both are extrapolations at long range; this is not a
  physical accuracy guarantee for ancient or future Earth orientation.
- [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html), six epochs
  between years 0 and 5000 and ten bodies: all 270 inter-body angular
  separations pass the predeclared diagnostic bounds. Maximum difference is
  `0.000746851 arcsecond` for pairs involving the Moon and
  `0.000005570 arcsecond` for the other pairs. Both calculations use DE441;
  separations are rotation-invariant, so this does **not** validate the
  precession, nutation or absolute ecliptic longitude.
- Horizons also provides absolute apparent ecliptic angles in its own
  IAU76/80-compatible long-range model. The largest absolute longitude
  differences for years 0, 1000, 2000, 3000, 4000 and 5000 respectively were
  approximately 3.03, 2.24, 0.055, 3.50, 7.93 and 12.15 arcseconds; maximum
  latitude difference across those cases was 0.224 arcsecond. These are
  **cross-model diagnostics**, not errors against a known truth or an accepted
  tolerance. The different frame models matter; Horizons documents its
  long-range orientation model. More epochs, rate comparisons, model and
  uncertainty review are required before accepting long-range accuracy.

The exact queries, reference data and results are stored locally under
`local-data/swiss-census-2026-11-17/`. Recreate the ERFA data with the two
`scripts/generate_long_term_*reference.py` scripts using caller-supplied
pyerfa 2.0.1.5, and run the ignored Rust tests with
`LONG_TERM_ERFA_REFERENCE` and `LONG_TERM_NUTATION_REFERENCE` set. The manual
Horizons comparator accepts `<kernel> <sha256> <new-output.json> apparent`.

## Remaining

Finish Swiss identity/designation/component and nonstellar coverage, assess
each candidate and catalog source, and test actual lookup semantics. Validate
the extended apparent model against additional independent references and
century/record boundaries, establish accepted accuracy and uncertainty,
address long-range Delta T and civil/calendar conventions, and separately
cover small bodies and chart/house calculations. No production switch.
