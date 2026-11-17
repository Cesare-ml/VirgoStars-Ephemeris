# Swiss alias screening and long-range planet grid — 2026-11-17

**Status: experimental/local. Full Swiss catalog coverage, stellar identity
review and long-range apparent accuracy remain open; production is not switched.**
This follows [the positional and UT1 checkpoint](SWISS_POSITIONAL_AND_EXPLICIT_UT1_2026-11-17.md).
For the subsequent 929-candidate catalog and ICRF comparison, see
[the HIP2 and ICRF checkpoint](HIP2_LEADS_AND_ICRF_2026-11-17.md).

## Swiss names, components and catalog alternatives

The local Swiss lookup uses prefix behavior for some names. In the earlier
706-star candidate manifest, all 404 alias strings were compared directly to
Swiss at 1950-05-15, 2000-01-01 and 2024-03-20 UT. `Atri` selected `Atria`
instead of the candidate Megrez, and `Men` selected `Menkalinan` instead of
the candidate `,alLup`; both aliases were withheld. Three-date agreement is
diagnostic evidence, not a scientific identification or exhaustive lookup test.

Re-running the strict positional crossmatch against the remaining Swiss rows
recovered six additional **designation-based candidates** after the existing
`omePhe` outlier was again withheld. These include components for which the
Swiss and historical IAU named entries differ. The aliases `Ran` and `Suhail`
were separately withheld because Swiss resolved those prefixes to `Rana` and
`Suhail Hadar`. The strict local manifest contains **712 stars and 405 aliases**;
all canonical names and aliases passed the one-arcsecond-per-component Swiss
comparison at the three sample epochs. Earlier quarantined identities remain
open; a matching designation does not establish the interpretation of a name.

The positional builder additionally accepts the explicit `traditional` policy
as an **opt-in alternative** to `strict`. It rechecks every base star against
the strict HIP2 policy before labeling it strict, and individually records
`qualityTier` for added stars. The traditional policy is less restrictive:
it accepts additional HIP2 solution types and up to two components while
retaining the existing parallax signal-to-noise threshold. It trades numerical
selection strictness for coverage and does not approve a candidate's identity.

With the same local NEC/HIP2/Swiss snapshots, this alternative produced 208
more candidate entries before screening. A comparison of all 920 candidates
against Swiss at the three epochs produced 2,760 comparisons: 2,759 were
within the one-arcsecond diagnostic bound; `,omePhe` remained outside it and
was withheld. An alias audit withheld `Nodus I`, which Swiss resolved to
`Nodus II`. The final **919-candidate experimental manifest** contains 712
strict-tier stars, 207 traditional-tier stars and 545 aliases; all retained
canonical entries and aliases met the bound at those three dates. It covers
549/729 distinct named entries and 915/1113 comma-prefixed designations in
the inventoried Swiss snapshot via case-insensitive exact lookups. These are
not complete Swiss lookup parity, independent data provenance, stellar
identity approval or proof of long-range stellar accuracy. All new and prior
entries still need review; zero radial velocity remains explicitly assumed.

Subsequently `extend-swiss-designation-lookups.cjs` tested every still-missing
comma-prefixed designation against the local Swiss lookup and all selected HIP2
stars on the same three epochs. A designation was added only when exactly one
already-selected star agreed within one arcsecond in both components at all
three dates. Eight additional candidate designations matched existing stars;
190 remained unresolved, including two unavailable from the reference.
This produces a separate **919-star, 553-alias** manifest, with exact lookup
coverage of **549/729** names and **923/1113** designations in this particular
Swiss snapshot. It does not settle physical identity or the behavior of
unmatched and ambiguous names. No Swiss astrometric values are stored in the
manifest; only caller-supplied lookup strings and source/diagnostic metadata
are added.

`audit-swiss-name-gaps.cjs` then classified the 180 traditional names still
absent from that manifest, again using actual Swiss lookup results and three
epochs. It found 162 with no selected matching position, ten for which Swiss
returned a different name (including prefix collisions), and eight matching
the position of an existing candidate. The latter are **review leads, not
automatically added aliases**: in particular Swiss's `Fafnir` and `Ukdah`
matches conflict with HIP identities assigned to those names by the local 2022
IAU snapshot. The complete categorized report is
`name-gaps-diagnostic.json` in the local ignored data directory.

The optional `qualityTier` in a manifest is carried through the prepared
catalog as a **caller assertion**, not independently proven by the parser. The
local experimental backend reports the tier and `CALLER_ASSERTED` status in
single-star and batch responses, and reports tiers by legacy name in the
conjunction response. With the 919-star manifest, the local loopback tests
exercise all 919 in natal and batch flows and 71 legacy names representing
70 catalog entries in the conjunction flow. No production server, real COW
database or provider has been switched. The 712-star strict manifest remains
available independently; choosing the alternative is explicit.

Reproduction requires caller-supplied catalog files and the existing compiled
addon. All outputs must be new paths:

```text
node scripts/extend_star_manifest_positions.mjs <strict-manifest> <sefstars> <nec.csv> <hip2> <new-manifest> <new-report> traditional
node bindings/node/scripts/audit-positional-swiss.cjs <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-report> all
node scripts/filter_positional_swiss_diagnostic.mjs <manifest> <star-report> <new-manifest>
node bindings/node/scripts/audit-swiss-aliases.cjs <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-report>
node scripts/filter_swiss_alias_diagnostic.mjs <manifest> <alias-report> <new-manifest>
node bindings/node/scripts/extend-swiss-designation-lookups.cjs <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-manifest> <new-report>
node bindings/node/scripts/audit-swiss-name-gaps.cjs <manifest> <hip2> <de441> <sha256> <swisseph> <sefstars> <new-report>
```

The resulting local files and detailed failures are in ignored
`local-data/swiss-census-2026-11-17/`, including
`manifest-round2-screened.json`, `manifest-traditional-screened.json`,
`manifest-designation-extended.json`, `designation-extension-report.json`,
`stars-traditional-round1.json`, `aliases-traditional-round1.json` and
`coverage-traditional.json`. External source hashes are retained. The NEC
snapshot's provenance and redistribution conditions remain unverified.

## Long-range planet diagnostic grid

`bindings/node/scripts/compare-horizons-long-range-grid.cjs` compares the
isolated experimental apparent path with [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/manual.html)
on July 1 of astronomical years 0 through 5000, every 250 years: 21 epochs,
Sun, Moon, Mercury, Venus and the six Mars–Pluto system barycenters. Inputs
are TDB for this engine and approximately corresponding TT for Horizons. The
query explicitly selects Julian-date inputs and outputs and validates each
returned TT epoch. Each source identifies DE441 for target and Earth.

For the **945 pairwise body separations**, maximum discrepancies are
0.001105 arcsecond for pairs with the Moon and 0.000005570 arcsecond for the
others, below the predeclared diagnostic limits 0.05 and 0.01 arcsecond. This
rotation-invariant comparison checks the corrected directions but **cannot**
validate the chosen precession/nutation frame or absolute longitude.

Horizons' absolute apparent ecliptic uses a different IAU76/80-compatible
long-term model. Over the 210 body/epoch samples, maximum absolute difference
is about **12.154 arcseconds longitude** and **0.224 arcsecond latitude**.
Using Horizons apparent ecliptic positions at ±0.01 day to estimate rates,
maximum component differences are **0.0169 arcsecond/day longitude** and
**0.0556 arcsecond/day latitude**. These are finite-difference and cross-model
diagnostics, with no predeclared acceptance threshold for absolute positions
or rates. They do not establish physical truth, behavior between samples,
UT1/Delta T accuracy, or readiness to replace the production provider.

The complete local report is `horizons-grid-250y-jd-verified-v2.json` in the same
ignored directory. Run the script with `<de441.eph> <sha256> <new-report>`.
The Node UT1 tests also perturb supplied Delta T by ±60 seconds at six epochs
and check consistency with the computed TDB-day longitude rate; this is a
numerical sensitivity check, not a measured uncertainty of Earth rotation.

Remaining: finish missing Swiss stars/names/components and objects, investigate
withheld candidates, individually review the strict/traditional policy and
sources, establish accepted long-range planet and rate tolerances with further
independent references, and resolve civil/UT1/Delta T and chart-body coverage.
The main server remains gated by the owner's full-catalog requirement.
