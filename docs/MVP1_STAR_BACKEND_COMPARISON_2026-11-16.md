# Local fixed-star backend diagnostic — 2026-11-16

**Status: diagnostic only; neither catalog approval nor general compatibility
is claimed.** The comparison used 40 exact-name candidates from the historical
IAU snapshot which pass the strict HIP2 numerical filter, not a reviewed product
catalog. No Swiss catalog entries or raw reference rows were copied into Git.

## Experiment

The unchanged local `virgo-stars/backend/services/astrology/func.js` was loaded
with the experimental facade injected only for this process. For 40 candidates
and three fictional UT dates (1950-05-15, 2000-01-01 and 2024-03-20, at 12:00),
`getStella` was exercised at 12.5° E, 41.9° N with southern inversion disabled.
The engine used the caller-supplied HIP2 file, strict quality filter, the
historical IAU snapshot's V-band values for the local diagnostic, the classic
DE441 file and the default UT conversion policy. HIP2 contains no RV: every
candidate explicitly used the **assumed zero** value. This assumption and
catalog identities need independent review.

The local, separately installed `swisseph` 0.5.17 package used its own existing
star catalog and was configured for JPL mode with the same DE441 path; returned
flags were checked for JPL. This is a comparison with a legacy implementation,
not independent absolute truth. Its catalog, radial velocities and magnitudes
are not identical to the IAU/HIP2 inputs. Reference source and file provenance
remain to be reviewed; no data is distributed here. The diagnostic does not
claim that configuring a path and checking `rflag` independently authenticates
the kernel bytes read by both processes.

## Observations

All 120 attempted star/date cases produced finite compared positions and a
backend star/azimuth result; no engine, reference or backend error was recorded.
The largest component differences were **0.00892 arcsecond** in longitude and
**0.00313 arcsecond** in latitude; the largest V magnitude difference was
**0.12 magnitude**. These reflect different source data and models, not an
approved error budget or an assessment of absolute accuracy.

The direct longitude and latitude speed fields differed by up to **0.18489**
and **0.06265 arcsecond/day**, respectively. To investigate, the diagnostic
formed a centered derivative of the *reference positions* at JD ±0.01 day,
using the actual representable separation. The engine's rates differed from
that derivative by at most **2.53e−6** and **7.64e−7 arcsecond/day**; the
reference's own reported speeds differed from its position derivative by up to
**0.18489** and **0.06265 arcsecond/day**. This identifies a reference-field
compatibility issue to investigate; it does not by itself establish which rate
is scientifically correct. No threshold was approved before the experiment.

The returned `name` strings differed in all 120 cases: the local reference
returns a compound name/designation, while the facade defaults to the supplied
name. Since backend consumers may use that field as a key, the facade now
accepts an optional caller-owned `displayName`; `prepareStarCatalog` requires
an associated `displayNameSource`. Lookup remains based on the name/aliases;
the label is never inferred from Swiss. This enables an explicit migration
mapping but does not itself populate or approve one.

## Additional legacy input findings — 2026-11-17

The current legacy `getStella` derives the Julian hour from hours and minutes
but omits seconds, unlike `getPianetiCase`. Its truthiness check also treats a
finite longitude of exactly `0` degrees as an absent star. A local fictional
provider confirmed that `12:34:59` is passed to `swe_julday` as `12:34:00`,
and an otherwise finite zero-longitude result is rejected. These are backend
source/behavior findings, not measured astronomical discrepancies. The prior
120-case diagnostic used whole-minute dates and did not cover either case.
The isolated backend branch `codex/ephemeris-house-diagnostics-20261116` now
corrects those two cases and passes local regression tests, including one with
the real Ephemeris/HIP2 path. The production checkout and separate Astrolabio
copy have not acquired that change; wider consumer validation remains open.

## Reproduction and evidence

`bindings/node/scripts/audit-star-backend.mjs` is a local-only diagnostic. It
requires the independently supplied candidate and HIP2-quality reports, HIP2
and DE441 files, installed reference package and trusted backend module. It
checks candidate/quality/HIP2 hashes, loads only strict-eligible candidates,
and prints aggregates, never raw reference rows. A missing or failed attempted
comparison makes it exit nonzero; there is no built-in scientific tolerance.
Example placeholders must be
replaced by caller paths; it makes no network request:

```powershell
node bindings/node/scripts/audit-star-backend.mjs <candidates.json> <quality.json> <hip2.dat> <de441.eph> <local-swisseph-dir> <backend-func.js>
```

The aggregate output was retained locally under
`VirgoStarsData/star-catalog-review/2026-11-16`; SHA-256
`2ced692cf984c0d917806a1c34cf15f69c5145c1f565f2a2739efb23cd8e4e92`
for `star-backend-diagnostic-v2.json` (120 magnitude comparisons and no
missing reference magnitudes in this run).
The hash identifies the saved output bytes (including its original line
endings), not the provenance of the source data. A production migration still
requires all requested objects, source and rights review, component-specific
policy, agreed tolerances, independent scientific checks and a representative
backend parity study.
