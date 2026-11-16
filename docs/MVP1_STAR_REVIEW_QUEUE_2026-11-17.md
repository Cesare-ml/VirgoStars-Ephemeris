# Fixed-star review queue — 2026-11-17

**Status: REVIEW_REQUIRED; not a runtime catalog.** The queue consolidates
existing identification leads and numerical HIP2 screens; every identity,
quality, magnitude, radial velocity, alias and data-rights decision remains
null. No real manifest is created or approved.

## Reconstructed local evidence

`scripts/build_star_review_queue.mjs` replays the historical IAU candidate
parser against the original name list and snapshot, checks their hashes and
the candidate/quality report linkage, then re-runs the HIP2 quality inspection
for each eligible unambiguous candidate. With an explicitly supplied SIMBAD
response and its quality report, it checks their hash linkage and repeats that
HIP2 inspection as well. It rejects inconsistent input, duplicates within a
quality set and unsupported CSV, and flags shared HIP candidates across names
instead of merging them. It refuses to overwrite an output file.

The local run combined **106 requested names**, **77 IAU exact-name leads**,
**4 SIMBAD exact-identifier leads**, and **25 names without either exact lead**.
Of the SIMBAD leads, three have a HIP candidate and one has no HIP in the saved
response. The counts are source-screen results, not accepted stars. The IAU
quality replay produced 40 strict and 63 traditional eligible cases; the SIMBAD
replay produced zero strict and one traditional eligible case. These eligibility
figures do not grant approval. All 106 entries have six unset decision fields.

The deterministic local queue is stored outside Git at
`VirgoStarsData/star-catalog-review/2026-11-17/star-review-queue-v1.json`;
SHA-256 `84535ec88183d81756057883b5f02b9a4bfa2b2b1496beefc1dc08ee175d031e`.
It contains caller application names and derived third-party identifiers; its
redistribution remains unreviewed. Five focused tests passed, including a real
local-source replay, byte-for-byte reproducibility and refusal to overwrite.

```powershell
node scripts/build_star_review_queue.mjs <requested-names.json> <iau-snapshot.txt> `
  <iau-candidates.json> <iau-quality.json> <hip2.dat> <new-queue.json> `
  <simbad-response.csv> <simbad-quality.json>
```

Both SIMBAD arguments may instead be omitted together. The SIMBAD parser is
deliberately limited to the exact four-column format of this local response;
unsupported quoting or rows fail rather than being guessed. File checks detect
observable changed bytes, not authenticated provenance, independent validation
or an atomic snapshot. The quality replay uses the **same Rust implementation**
as the original screening. The command does not infer RV values or evidence of
legal permission and does not create a `virgostars-star-manifest/1`.

## Two separately sourced spelling leads

These are not yet incorporated into the machine-generated exact-match queue.
In [IAU WGSN Bulletin 1](https://www.iau.org/static/science/scientific_bodies/working_groups/280/WGSN_bulletin1.pdf),
the table and its spelling footnotes associate the following historic spellings
with standardized names:

| Backend input | IAU table name and designation | Local 2022 snapshot candidate | Local HIP2 screen |
|---|---|---|---|
| El Nath | Elnath, HR 1791 | HIP 25428, component Aa | strict and traditional eligible |
| Ras Alhague | Rasalhague, HR 6556 | HIP 86032, component A | strict ineligible; traditional eligible |

The HIP/component candidate comes from the historical local snapshot, not from
the bulletin's spelling footnote. As a separate cross-identification check,
[SIMBAD β Tau](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=beta+tau&submit=submit+id)
lists HR 1791 and HIP 25428, and [SIMBAD α Oph](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=ALF+OPH&submit=submit+id)
lists HR 6556 and HIP 86032. SIMBAD identifies α Oph as a multiple system;
these identifiers alone do not settle which component or combined-light value
the backend intends. Both leads need component/alias review,
astrometry and photometry assessment, radial-velocity sourcing or explicit
assumption, and data-rights review. In particular, a traditional-eligible
screen does not extend an earlier approval for other named stars. These two
are evidence-backed leads within the 25 that lacked an **exact** IAU/SIMBAD
match; they do not change the queue's exact-match counts or establish a
production mapping.

## Next decisions

1. Resolve each object and component, including ambiguous/multiple objects and
   the no-HIP cluster; verify aliases against independent identification sources.
2. Record HIP2 quality policy decisions per object, reviewed photometry,
   radial velocities or explicit assumptions, and permitted date ranges.
3. Complete a data-specific rights, attribution and update-policy review, then
   construct and validate a separate caller-reviewed manifest.
4. Re-run the actual backend star workflow and independent numerical checks on
   the resulting reviewed data before any product or release decision.
