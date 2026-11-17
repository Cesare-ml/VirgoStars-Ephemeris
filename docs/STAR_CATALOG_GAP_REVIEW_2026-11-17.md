# Stellar lookup gap review — 2026-11-17

**Follow-up:** the [component and designation review](STAR_COMPONENT_DESIGNATIONS_2026-11-17.md)
adds six local designations with explicit identity and compatibility notes.

**Experimental, caller-asserted candidates only.** This continues the
[catalog recovery](STAR_CATALOG_RECOVERY_2026-11-17.md). No main-server
activation, full-catalog approval or scientific accuracy guarantee is implied.

## Six newly selected HIP2 entries

The previous local manifest's remaining source rows were screened against
HIP2 and a caller-supplied IAU-WGSN NEC snapshot; HIP quality was rechecked
using the actual Rust-backed inspection API. Magnitudes below are local
Hipparcos I/239 V values. All six explicitly assume zero radial velocity.
The quality tier is the engine's filter result, not an individual validation.

| Lookup(s) | HIP | Tier | Explicit limitation or discrepancy |
| --- | ---: | --- | --- |
| Alkurhah, Kurdah, `,xiCep` | 108917 | experimental | IAU calls ξ Cep Aa **Kurhah**, but the HIP solution covers a multiple system and is not a resolved Aa orbit. IAU/Hipparcos V 4.26; local Swiss reports 6.50. |
| `,ka-2Lup` | 74380 | experimental | Hipparcos V 5.70; local Swiss reports 5.498. HIP2's five-parameter approximation does not model additional orbital terms. |
| `,upPhe` | 5300 | experimental | A multiple-star solution; latitude differed by up to 1.339″ on the nine diagnostic epochs below. |
| `,omePhe` | 4829 | strict | Longitude differed by up to 2.680″ on those epochs; this earlier withheld outlier is retained only with a warning. |
| `,rhHer` | 85112 | experimental | A multiple-system solution, not a resolved component orbit. Hipparcos V 4.15; local Swiss reports 4.56. |
| `,yEri` | 9767 | experimental | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=Y+Eri) identifies HIP 9767 as Y Eri; Hipparcos V 9.69 versus local Swiss 10.68. It is variable: the value is a fixed catalog magnitude, not a light curve. |

The [IAU WGSN bulletin](https://www.iau.org/static/science/scientific_bodies/working_groups/280/WGSN_bulletin2.pdf)
identifies Kurhah as ξ Cep; [SIMBAD ξ Cep](https://simbad.u-strasbg.fr/simbad/sim-basic?Ident=xi.+Cep)
associates it with HIP 108917, and the [RASC double-star log](https://www.rasc.ca/sites/default/files/RASC%20DS%20log%20book%20210411.pdf)
records Alkurhah as an alternative. The local Swiss input contains both
Alkurhah and Kurdah for the same designation; the latter is retained as a
local name claim, not a new IAU name. The locally supplied IAU-NEC crossmatch
provides the other Bayer/HIP associations. Component identity, binary motion,
photometric conventions and all-date accuracy remain open, even for a strict
filter result.

`SWISS_MAGNITUDE_MISMATCH` is a new opt-in, per-lookup warning code. The
facade carries it through both star-position and magnitude responses. The
caller supplies its detail; the code does not automatically measure or validate
magnitudes. Existing warning codes and lookup behavior remain unchanged.

## Local measurements

The output `manifest-gap-six-20261117.json` contains 1,069 candidates and
696 aliases. The exact-string gate finds **692/729 names** and **1,073/1,113
designations**: **37 names and 40 designations remain**. These counts do not
correspond to distinct physical objects or imply complete Swiss compatibility.
There are no null magnitudes; 1,064 candidates assume zero radial velocity.

The full local Swiss JPL-mode position check ran all 1,765 lookup strings on
nine dates, May 15 from 1900 to 2099. All **15,885** comparisons completed;
207 exceeded one arcsecond in at least one component, all carrying a
predeclared position warning. The engine and reference share DE441, and Swiss
received TT while the engine used approximately corresponding TDB. The check
is sampled comparison, not absolute accuracy, identity proof, a magnitude
validation or a guarantee between dates. A separate local facade smoke check
confirmed each new lookup returns the expected caller-asserted warning.

The manifest's SHA-256 is
`f714cdea1a16208d08f737585a4ce5ff69b8bb2db6252fa4771c7ee257d6740c`;
the coverage report's is
`68291932ca99fc3b4e8b1b08d274aac455f6a6be3b674e228aca09cd39720ea7`;
the century report's is
`e137dc657b4410bcfe96fd2d6d49f170f5a0f80177c0e7236e08c9fdbceae647`.
These files and catalog sources are local ignored artifacts, not in Git.

The two input decision documents have SHA-256
`e45a662799de23e0f67908abb29dad303a804701ce3cc77f4f3d7b03b70c3983` and
`5d265d580e45a3871018efb9e59303cc78b21cacf2edd506baf95bb1162996d6`.
Reproduce with `scripts/add_reviewed_hip_stars.mjs` twice using the prior
manifest and then the intermediate manifest, followed by
`scripts/verify_swiss_coverage.mjs` and
`bindings/node/scripts/audit-star-century.cjs` as described in the recovery
checkpoint. Scripts verify bytes and supplied quality decisions; they do not
authenticate the caller's source or settle scientific identity.

Remaining entries include reference directions, extended objects, shared
names and stars without an eligible HIP2 solution. Each needs an appropriate
representation and source review; do not substitute an unrelated star or
invent a parallax solely to close the lookup gate.
