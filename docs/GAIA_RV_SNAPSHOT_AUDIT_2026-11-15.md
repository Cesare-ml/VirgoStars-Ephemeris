# Offline Gaia radial-velocity snapshot audit

This operator-only utility audits locally supplied SIMBAD HIP↔Gaia DR3
crossmatch TSV and one VizieR I/355 `gaiadr3` TSV snapshot per HIP identifier.
It does not download data, authenticate snapshots, establish data rights,
verify a crossmatch, approve a stellar solution or install a catalog. The
Gaia DR3 `radial_velocity` field is spectroscopic and referred to the solar
system barycenter, according to the [Gaia DR3 data model](https://gea.esac.esa.int/archive/documentation/GDR3/Gaia_archive/chap_datamodel/sec_dm_main_source_catalogue/ssec_dm_gaia_source.html).
Measurements from other sources must not silently be treated as equivalent.

The crossmatch input must have the exact SIMBAD TAP TSV columns `hip_id` and
`gaia_id`, with identifiers of the form `"HIP 123"` and `"Gaia DR3 ..."`.
The Gaia snapshots must be named `<HIP>.tsv` and contain the VizieR table
`I/355/gaiadr3` with exactly `Source`, `RV`, `e_RV`, `RUWE` columns for the
matched source. Preserve 64-bit Gaia identifiers as decimal strings; JavaScript
numbers cannot represent them exactly. Ambiguous duplicate HIP or Gaia IDs,
malformed rows, mismatched IDs and invalid measurements are rejected. Missing
snapshots are reported separately from present snapshots whose RV is blank;
neither case is replaced with a zero velocity.

```powershell
node scripts/audit-gaia-rv-snapshots.mjs `
  --crossmatch="C:\VirgoStarsData\simbad-crossmatches.tsv" `
  --crossmatch-sha256="<trusted lowercase SHA-256>" `
  --gaia-dir="C:\VirgoStarsData\gaia-snapshots" `
  --out="C:\VirgoStarsData\gaia-rv-audit.json"
```

Output is exclusively created outside any Git worktree. The report pins the
crossmatch input and records a SHA-256 for each supplied snapshot; these
hashes identify bytes and **do not authenticate the publisher**. A present
RV is always `RV_PRESENT_REVIEW_REQUIRED`; RUWE is reported, not turned into
a pass/fail astrophysical verdict. The report always states
`scientificallyValidated:false`. Verify the identifiers, measurement epochs,
uncertainties, source license and possible multiplicity or variability before
using any value. The utility does not accept or infer a Johnson V magnitude.

## Local exploratory result

On 2026-11-15 a single batched query to the [CDS-listed SIMBAD Harvard mirror](https://simbad.cds.unistra.fr/)
returned 15 possible Gaia DR3 associations among 43 Hipparcos target records
that passed the earlier preliminary quality checks. Fifteen separately
obtained VizieR snapshots yielded one nonempty RV and 14 blank RV fields.
The one candidate had RUWE 2.811 and requires review; its V-band uncertainty
was not supplied by the cited SIMBAD photometry. This is an exploratory
coverage observation, not a statistical estimate, a vetted catalog or a
claim that the remaining targets have no usable RV in other sources. The
raw and derived snapshots were kept outside Git; no third-party data is
bundled. The source snapshot hashes are recorded in the local audit report.
