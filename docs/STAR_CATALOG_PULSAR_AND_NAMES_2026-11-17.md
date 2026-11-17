# Pulsar and historical names — local review, 2026-11-17

For subsequent local candidates see
[STAR_CATALOG_K01PUP_ETACAR_2026-11-17.md](STAR_CATALOG_K01PUP_ETACAR_2026-11-17.md).

**Experimental caller-supplied catalog; not a production release.** Names and
measurements below are local assertions, not bundled third-party datasets.

## Identity decisions and Swiss differences

| Lookup | Local identity | Locally observed Swiss behavior |
| --- | --- | --- |
| `Ruc` | Historical name for delta Cygni, the existing HIP 97165 entry; [Astrodienst](https://www.astro.com/astrowiki/en/List_of_Fixed_Stars) lists Ruc alongside Rukh. The simplified HIP2 model does not model the full multiple system. | `swe_fixstar("Ruc", ...)` returned Ruchbah, delta Cassiopeiae. The local lookup warns of the different position. |
| `Men` | Historical name associated with alpha Lupi, the existing HIP 71860 entry; [name discussion](https://www.star-facts.com/alpha-lupi/). Historical naming and binary motion merit further review. | `swe_fixstar("Men", ...)` returned Menkalinan, beta Aurigae. The local lookup warns of the different position. |
| `Lich`, `,PSRB1257+12` | Pulsar PSR B1257+12, not its orbiting planets; [IAU](https://www.iau.org/static/science/scientific_bodies/working_groups/280/wg-starnames-annual-report-2019.pdf), [SIMBAD](https://simbad.cds.unistra.fr/simbad/sim-id?Ident=PSR+B1257%2B12). | Swiss returned the pulsar name, but magnitude 999.99. The local catalog does not treat that sentinel as a visual measurement and returns `E_STAR_MAGNITUDE` with a caller-asserted warning. |

The name results were observed with the installed Swiss binding and local
catalog, not established as behavior of every Swiss version. Name matching
and physical identities are distinct from numerical accuracy.

The Lich point source uses the SIMBAD ICRS J2000 position and its cited proper
motion and parallax (1.41 ± 0.08 mas). J2000 is approximated as TDB JD
2451545.0; the radial velocity is explicitly assumed zero. SIMBAD labels the
position quality D. The simplified linear model does not propagate
measurement uncertainties or model radio propagation, pulsar orbital effects
or relativistic timing. These choices cannot justify long-range precision.

When a catalog lookup has a declared warning and its magnitude is unavailable,
the facade now preserves that warning in the `error` text. The error still
has only the enumerable `error` key, the same non-enumerable error `code`, and
the same synchronous callback behavior. An unflagged lookup gets the previous
error text. Warnings are caller assertions, not authenticated conclusions.

## Local validation

The local manifest has **1,081 objects and 709 aliases**, covering **701/729
names and 1,089/1,113 designations**. There remain 28 missing names, 24 missing
designations, five objects with unavailable magnitude and 1,068 objects with
an assumed zero radial velocity. Exact-string coverage is not physical or
photometric completeness.

The same-DE441 nine-epoch comparison from 1900 through 2099 attempted 16,110
lookup/epoch cases: 16,092 numerical comparisons and 18 unavailable Swiss
references. There were 324 cases with at least one coordinate over the 1 arcsec
diagnostic threshold and 18 unavailable references; all 342 cases had declared
warnings. That threshold is not an approved accuracy requirement. This is not
an independent reference: both implementations use DE441, while Swiss uses TT
and the engine an approximate TDB conversion. Large warned identity differences
are intentional and are not evidence of a successful accuracy test.

Reproduce from the preceding local manifest with
`scripts/attach_reviewed_star_aliases.mjs`, then
`scripts/attach_explicit_star_candidates.mjs`, then run
`scripts/verify_swiss_coverage.mjs` and
`bindings/node/scripts/audit-star-century.cjs` with fresh output paths. The
decision files are hash-bound to their asserted inputs; the hashes do not
authenticate the scientific source or validate the model.

SHA-256 of ignored local artifacts:

- `manifest-final-lich-20261117.json`: `7701c39ee621fa60693b3ef227b88f3b6f5295416383126753f88f4d5f9fae8e`.
- `coverage-final-lich-20261117.json`: `6b7b902ffb1a8f02f4502784ba8c3b1dc9f5b9a738f7a17f17c49cb329552d61`.
- `stars-century-final-lich-20261117.json`: `aafa6fc37ecb51c27b44fc8a3d33c973d33a9887a15c3a4a92fc597eef6758b8`.
- `decisions-ruc-men-warnings-20261117.json`: `7941fdb97901bd30c5e22723fc484ca30396c8503aea7811cb1e6790ec8a75b5`.
- `explicit-lich-final-20261117.json`: `ef9f4a6b8e6d7f5db7218e743adb9177ffec4ade3cfa0f06a4f7242c7caea0e7`.

The remaining names and designations, unresolved component identities,
photometry and independent error budgets still require work before production
integration. No server configuration has been changed.
