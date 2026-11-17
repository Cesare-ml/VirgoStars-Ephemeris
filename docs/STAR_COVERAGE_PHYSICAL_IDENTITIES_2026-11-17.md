# Stellar coverage and physical identities — 2026-11-17

**EXPERIMENTAL, LOCAL ONLY. Stellar coverage is incomplete.** The owner chose
the astronomically correct identity when Swiss uses a different object, with
an explicit notice for the affected lookup. The production server remains
unchanged; the owner requires complete Swiss catalog coverage before any
production replacement.

## Local catalog and lookup policy

The current caller-owned manifest is
`local-data/swiss-census-2026-11-17/manifest-iau-near-aliases.json`.
It is ignored local data, not a bundled or published catalog. It contains
1,062 candidate objects and 690 aliases, with 687/729 Swiss name strings
and 1,065/1,113 comma-prefixed designations represented. The
`scripts/verify_swiss_coverage.mjs` gate reports **INCOMPLETE** (42 missing
names, 48 missing designations); it checks exact lookup strings and is not
scientific or deployment approval. Input fingerprints and detailed unresolved
names are recorded in ignored `coverage-gate-near-aliases.json`.

The optional `experimental` HIP2 policy is explicitly selected, separate from
the strict and traditional policies. It can accept positive low-S/N
parallaxes, binary, photocenter, stochastic and other nonstandard solutions.
Its simplified propagation uses the catalog's linear five parameters and
does **not** apply extra acceleration, orbital or variability parameters.
Each star retains its caller-asserted quality tier; a successful positional
comparison does not prove a physical component identity or long-term model
accuracy. The local [ESA HIP2 description](https://gea.esac.esa.int/archive/documentation/GDR2/Gaia_archive/chap_datamodel/sec_dm_external_catalogues/ssec_dm_hipparcos_newreduction.html)
defines these solution classifications.

The caller-provided Hipparcos original main catalog (CDS I/239, local Harvard
mirror `https://vizier.cfa.harvard.edu/ftp/cats/I/239/hip_main.dat`) was
used for independent V magnitudes and position cross-checks. The downloaded
local file is 53,316,318 bytes, 118,218 rows, SHA-256
`58ceabb104d647160d9437ce6e513a02a036bb4ad9f8879a5a22fd52943616e0`;
the data are not committed. A candidate needs unique close Swiss/HIP2 position
and consistent HIP1 epoch position/magnitude, followed by a Swiss comparison.
Conflicting or incomplete cases are withheld.

The independently sourced [ESA Hipparcos volume 1, table 1.2.3](https://vizier.cfa.harvard.edu/ftp/cats/more/HIP/cdroms/docs/vol1/sect1_02.pdf)
supplies historical radial velocities for Barnard's star, 61 Cygni A,
and three Alpha/Proxima Centauri entries; these are explicitly attributed,
not claimed to be current measurements. Other null radial velocities remain
explicitly assumed zero.

Exact IAU-name/HIP matches add correct physical identities including
Proxima Centauri and Fafnir. Aliases based on the same source astrometry are
added only when anchored to a matching IAU name and cross-checked, with
conflicts and ambiguous cases withheld. A `lookupWarnings` entry applies
to its specific lookup string; the facade and local backend surface it as
`compatibilityWarning` with status `CALLER_ASSERTED`. This includes both
different returned objects and numerical differences from Swiss; it does not
assert that every difference has the same cause. Existing names with no
warning retain their old response shape.

Mabsuthat, Miz0 and Alc0 are additionally attached to existing IAU-anchored
entries only after verifying close source positions, proper motions and
magnitudes and agreement with Swiss at three dates. Multiple distinct
components without a unique anchor remain unresolved.

## Objects without HIP identifiers

Manifest schema `virgostars-star-manifest/2` adds explicit, caller-supplied
ICRS astrometry with a unique source identifier, epoch in TDB JD, position,
proper motion, positive finite parallax, magnitude and stated radial velocity
or explicit zero assumption. Only the opt-in experimental policy accepts
these entries. The Rust core performs the same star calculation; Node is a
thin adapter. Schema /1 and its HIP behavior remain supported.

The local /2 catalog adds TRAPPIST-1 using Gaia DR3/JWST J2016 parameters
from [STScI's JWST target listing](https://www.stsci.edu/jwst/phase2-public/9256.pdf)
and V magnitude from the [ESO discovery paper](https://www.eso.org/public/archives/releases/sciencepapers/eso1615/eso1615a.pdf).
The target-list coordinates are rounded; J2016 is treated as a TDB epoch
approximation and no radial velocity is supplied. This is a local candidate,
not an accuracy certification. The external source facts remain in ignored
caller-owned data. Finite-distance direct entries still require
`0.001 ≤ parallax ≤ 10,000 mas`; no fake distance is manufactured for
zero or negative parallax. Galaxies, clusters, reference points, nonexistent
or ambiguous objects therefore need separate explicit models and evidence.

## Evidence and open work

The actual local manifest was checked against Swiss JPL-mode over nine TT
epochs (May 15, 1900–2099) for all 1,752 names: 15,768 comparisons. The
engine uses approximately corresponding TDB epochs. There were 174 cases
above 1″, all associated with 26 caller-asserted warnings on affected
lookups; there were no *unmarked* differences above that diagnostic threshold.
Those numbers do not make the discrepancies acceptable, prove the IAU
identity for every candidate, or establish accuracy between samples.

Barnard's star and Acamar, for example, exceed the diagnostic threshold
at some edge epochs despite closer agreement on the original three samples;
their warnings are retained. Proxima Centauri and other names intentionally
resolve to the IAU-selected object rather than Swiss's different result.
The local report `stars-century-near-aliases.json` contains the cases.
The existing 1900–2100 orientation range, occultation errors, missing radial
velocities and simplified binary dynamics remain limits.

Node/Rust unit and catalog tests, the native addon build, and a local loopback
backend test cover the mixed catalog and warnings; they do not prove CI,
staging, production activation or all-star physical identity. Remaining:
resolve the 42 names and 48 designations against independent sources, design
the nonstellar/direction-only paths, individually review the broad HIP2 cases,
and define model- and epoch-specific accuracy requirements. Repeat the full
coverage gate and compatibility tests before considering integration.
