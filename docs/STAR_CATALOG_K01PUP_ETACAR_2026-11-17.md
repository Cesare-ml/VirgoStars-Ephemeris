# Two stellar designations — local review, 2026-11-17

**Experimental caller-supplied catalog; no production activation.** This is a
follow-up to [the pulsar and names checkpoint](STAR_CATALOG_PULSAR_AND_NAMES_2026-11-17.md).
The local manifests, decisions, original catalogs and kernels remain outside
Git. Lookup coverage does not prove physical or numerical correctness.

## Candidates

| Lookup | Identity and model | Local Swiss comparison |
| --- | --- | --- |
| `,k01Pup` | [HD 61555 / k¹ Puppis](https://simbad.cds.unistra.fr/simbad/sim-id?Ident=HD+61555), also listed as WDS J07388-2648B. The local candidate uses the Gaia EDR3 astrometry displayed by SIMBAD and catalog V=4.50. | Swiss magnitude is also 4.50. Differences in the underlying astrometry are warned; in the sampled dates a longitude component reached 1.100 arcseconds in 2099. |
| `Foramen`, `,etCar` | Reference point for the unresolved [Eta Carinae system](https://simbad.u-strasbg.fr/simbad/sim-basic?Ident=eta+car), historically called [Foramen](https://www.astro.com/astrowiki/en/Foramen); it is not a resolved binary orbit or model of the surrounding nebula. | Swiss returns a fixed magnitude 6.21. Because the source is highly variable, the local magnitude is unavailable and the error includes a warning. Sampled position components differ by less than the one-arcsecond diagnostic threshold. |

For k¹ Puppis, the ICRS J2000 position shown by SIMBAD is used at approximate
TDB JD 2451545.0 with Gaia proper motion and parallax; an older heliocentric
radial velocity is treated as approximately barycentric. The position and
velocity are from heterogeneous epochs and sources; the simple linear model
does not represent possible orbital motion or propagate uncertainties. Gaia
measurements and a matching magnitude do not establish precision over centuries.

For Eta Carinae, the catalog position and proper motion are combined with an
approximate [2.3 kpc distance](https://ntrs.nasa.gov/api/citations/20230004225/downloads/Gull_2022_ApJ_933_175.pdf),
represented as 0.435 mas inverse-distance parallax; this is an assumption,
not a directly measured trigonometric parallax. An older heliocentric radial
velocity is approximated as barycentric. The J2000 reference point, linear
motion, uncertain radial velocity and distance are simplified and do not model
the binary, its surrounding nebula, or the time-varying light. The candidate
does not assert an error bound or a time-specific magnitude. The same caveat
applies when Swiss reports a fixed snapshot magnitude.

## Coverage and diagnostic

The local manifest contains **1,083 objects and 710 aliases**. It covers
**702/729 names and 1,091/1,113 designations**, leaving 27 names and 22
designations. Six objects have no asserted magnitude and 1,068 assume zero
radial velocity. These counts are exact strings, not a count of validated
stellar systems.

The all-name nine-epoch 1900–2099 diagnostic attempted **16,137** cases:
**16,119** numerical comparisons and **18** unavailable Swiss references.
There were **325 cases with at least one coordinate over the 1 arcsecond
diagnostic threshold** and 18 unavailable references; all 343 had declared
warnings. This comparison shares DE441 and uses TT in Swiss versus an
approximate TDB conversion here. It is not an independent error measurement,
and the threshold is not an accuracy requirement. A real-engine facade smoke
check exercised the three new lookups, warnings and missing-magnitude errors.

Reproduce with fresh output paths: apply `explicit-k01pup-20261117.json` to
`manifest-final-lich-20261117.json`, then apply `explicit-etacar-20261117.json`
using `scripts/attach_explicit_star_candidates.mjs`. Run the coverage gate and
the century audit described in the preceding checkpoint. Hash binding detects
changed input bytes, not wrong units, identities or scientific sources.

SHA-256 of ignored local artifacts:

- `manifest-etacar-k01pup-20261117.json`: `908da6937d255c5b5dc6c23395823953d528c9ff5752ad966a9d6d93c2115065`.
- `coverage-etacar-k01pup-20261117.json`: `ad31070fbf97d2cda7c3158d41164d3a98a660a853fe4c6bc81cda5d8f8b2b36`.
- `stars-century-etacar-k01pup-20261117.json`: `fdf603b1321642cb9ec55d867ee5c4b10166f40357574add7c65f7c1c9b6e5a7`.
- `explicit-k01pup-20261117.json`: `9766265eddf603be0185c9e6809ebd3b9b1afab937fb80392ff91134013497e0`.
- `explicit-etacar-20261117.json`: `c023d4f19b1f15ce79e911545c8d1a7ac1deb2871365d6dbca1f6c4d1c0afbfa`.

Other missing designations include unresolved multiple systems and nonstellar
objects; continue their identity and uncertainty review before claiming full
coverage or integrating into production.
