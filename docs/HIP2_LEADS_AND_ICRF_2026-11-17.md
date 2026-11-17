# Hipparcos leads and apparent ICRF diagnostic — 2026-11-17

Status: **EXPERIMENTAL, LOCAL ONLY**. No production integration or activation.
The owner requires complete Swiss star coverage before the main server can use
this engine. This checkpoint does not satisfy that requirement.
For the subsequent emission refinement and additional lookup names, see
[the follow-up](EMISSION_REFINEMENT_AND_NAMES_2026-11-17.md).

## Local stellar candidates

`scripts/audit_hip2_swiss_unmatched.mjs` searches the entire caller-supplied
Hipparcos new-reduction catalog for missing Swiss ICRS designations. It
propagates HIP2's ICRS coordinates from J1991.25 to J2000 using proper motion
and performs a one-arcsecond positional search. A positional lead is not proof
of identity. The local run found 150 single-HIP positional leads among 183
eligible unmatched designation groups. Some groups lead to the same HIP.

`scripts/extend_star_manifest_hip2_leads.mjs` requires a closer match
(0.05 arcsecond), an accepted traditional/strict HIP2 quality solution, one
independent NEC photometric row for that HIP, and agreement in magnitude to
0.1. Proper names are attached only if NEC assigns that same name to the HIP;
designations sharing a HIP are combined. Selection remains a research aid,
not individual approval. Source hashes are checked and external data stay in
the ignored local directory. Missing radial velocity is explicitly zero;
the NEC snapshot's provenance and redistribution conditions remain unverified.

Eleven new candidates were selected. The existing three-epoch Swiss JPL-mode
diagnostic (1950, 2000, 2024, longitude and latitude, one arcsecond threshold)
withheld one (`,omePhe`, HIP 4829; 1.3124 arcsec longitude in 1950). Ten
remain, including Mira, Alkalurops, Dabih, Errai, Copernicus and Secunda
Hyadum. All 562 aliases in the resulting 929-candidate local manifest passed
the same three-epoch diagnostic (1,686 comparisons). This is a numerical
comparison and does not prove identity or accuracy beyond those epochs.

Coverage against the same local Swiss snapshot is now 555 of 729 distinct
named lookup strings and 936 of 1,113 distinct comma-prefixed designations;
174 names and 177 designations are still missing. These are lookup counts,
not independent star counts. The previous 919-entry manifest is preserved.
The new local manifest is
`local-data/swiss-census-2026-11-17/manifest-hip2-stars-screened.json`;
its source report and diagnostic results are adjacent. The backend's
experimental loopback test loaded all 929 entries into the batch and natal
flows and exercised the Chiron conjunction path. This is not production.

## Apparent inertial directions

The isolated `experimentalApparentIcrfTdb` method exposes the same
Earth-centered, light-time, solar-deflection and annual-aberration calculation
in fixed ICRF axes, without Earth orientation, atmosphere or civil-time
conversion. Input is Julian date TDB, position is in km and velocity is its
derivative in km/TDB day. Position magnitude is the light-time range. For
Mars through Pluto, targets are DE441 system barycenters. It does not change
`calcTdb`, `calcUt`, the long-term precession model, or the deployed backend.

`bindings/node/scripts/compare-horizons-apparent-icrf.cjs` compares these
airless directions with [NASA JPL Horizons quantity 45](https://ssd.jpl.nasa.gov/horizons/manual.html)
at 21 July 1 epochs, astronomical years 0–5000 in 250-year steps, for the Sun,
Moon, Mercury, Venus and six outer-system barycenters. The script checks the
JPL target, Earth center, DE441 source and every returned TT epoch; TDB/TT is
converted approximately as in the earlier grid. The predeclared *diagnostic*
limits were 0.05 arcsecond for the Moon and 0.01 for the other bodies.
The 210 comparisons passed: maxima were 0.0011052 arcsecond for the Moon and
0.000004205 arcsecond for the others. The full raw responses and settings
are stored locally in `horizons-apparent-icrf-grid.json`.

Both implementations use DE441: this tests computational implementation and
apparent corrections, not independent orbital truth. This inertial comparison
also cannot establish the accuracy of the date-dependent ecliptic frame. The
previous ecliptic comparison still differs from Horizons by up to 12.154
arcseconds in longitude at the sampled epochs due to differing models; the
new diagnostic **does not remove that discrepancy**. Between-sample behavior,
uncertainty in DE441 over the full range, accepted product tolerances, UT1 and
Delta T remain unresolved.

## Reproduction and next work

Run the two stellar scripts with caller-supplied manifest, Swiss, HIP2 and NEC
paths, then `audit-positional-swiss.cjs`,
`filter_positional_swiss_diagnostic.mjs` and `audit-swiss-aliases.cjs`;
all report/manifest output paths must be new. Run the Horizons script with
`<de441> <sha256> <new-report>`; it makes explicit public NASA queries.

Continue reviewing omitted designations, ambiguous names, multiple systems,
poor HIP2 solutions and high-proper-motion stars. Establish a suitable
long-range ecliptic reference and product accuracy tolerances, including
uncertainties and velocity/UT1 effects. Do not switch the production provider.
