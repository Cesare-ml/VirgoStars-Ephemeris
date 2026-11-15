# Exploratory comparison with Gaia J2016 ICRS position

`compare-gaia-position-icrs.mjs` compares one classical barycentric ICRS
direction propagated by the Rust core from an externally supplied Hipparcos
catalog entry with one locally supplied Gaia DR3 position at J2016.0. It is
an **observational diagnostic**, not a catalog approval or automated accuracy
pass/fail. No external data or network request is bundled.

Inputs are an external catalog plus its importer audit, a SIMBAD↔Gaia identity
audit produced by [the snapshot audit](GAIA_RV_SNAPSHOT_AUDIT_2026-11-15.md),
and a VizieR `I/355/gaiadr3` TSV snapshot. All inputs require SHA-256 pins and
identities must agree. The Gaia snapshot must explicitly identify the exact
source, ICRS J2016.0, and contain columns `Source`, `RA_ICRS`, `DE_ICRS`,
`e_RA_ICRS`, `e_DE_ICRS`, `Plx`, `e_Plx`, `pmRA`, `pmDE`, `RV`, `e_RV`,
`RUWE` in that order. Gaia source IDs remain strings, avoiding integer rounding.
Checks and hashes do not authenticate sources, certify identity or grant data
rights. The output is exclusively created outside Git and remains
`scientificallyValidated:false`.

```powershell
node scripts/compare-gaia-position-icrs.mjs `
  --catalog="C:\VirgoStarsData\catalog.json" --catalog-sha256="<sha256>" `
  --catalog-audit="C:\VirgoStarsData\catalog-audit.json" --catalog-audit-sha256="<sha256>" `
  --identity-audit="C:\VirgoStarsData\gaia-rv-audit.json" --identity-audit-sha256="<sha256>" `
  --gaia-observed="C:\VirgoStarsData\gaia-j2016.tsv" --gaia-observed-sha256="<sha256>" `
  --hip=57757 --executable="C:\VirgoStarsData\true_ecliptic_at.exe" `
  --epoch-policy=TCB_JD_AS_TDB_JD_APPROXIMATION --out="C:\VirgoStarsData\position-diagnostic.json"
```

Gaia's reference epoch is a Julian year in TCB, as [the data model states](https://gea.esac.esa.int/archive/documentation/GEDR3/Gaia_archive/chap_datamodel/sec_dm_main_tables/ssec_dm_gaia_source.html).
This diagnostic explicitly treats its numeric J2016.0 JD as TDB rather than
implementing a TCB↔TDB conversion. The input Hipparcos epoch is likewise
interpreted under the importer’s TDB convention. The classical stellar motion
omits stellar light-time and relativity. Gaia and Hipparcos measurement errors,
covariances, crossmatch ambiguity, possible multiplicity and variability,
systematic errors, differences in models and photometric/RV source quality
need independent evaluation. Gaia formal errors and RUWE are reported as
published fields; they are not an error budget or an automatic quality verdict.

## Local exploratory pilot

One pilot used a HIP 57757 ↔ Gaia DR3 3796442680948579328 association from
SIMBAD, a local Hipparcos source, a provisional Gaia radial velocity and a
V magnitude with unreported uncertainty. Its Rust-propagated direction and
Gaia J2016.0 position differed by **0.01286114 arcseconds**. The Gaia RUWE was
2.811 and requires review. This is one candidate at one epoch, with the
approximations above; it does not establish a tolerance, accuracy bound, source
identity, catalog rights, observed-star certification, apparent/ecliptic
accuracy, or a production-ready result. Input snapshots and output reports
remain outside the repository. The numerical result may change with reviewed
sources, a proper time conversion and a full uncertainty analysis.
