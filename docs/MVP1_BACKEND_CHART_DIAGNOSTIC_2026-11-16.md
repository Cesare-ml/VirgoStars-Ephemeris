# Local backend chart diagnostic — 2026-11-16

**Status: diagnostic only, not parity acceptance or production activation.**
The test ran the unchanged backend module twice in one local Node process:
once with the experimental facade injected, once with the separately installed
`swisseph` 0.5.17 package. There were no database calls or personal birth data.

## Matrix and source

Three fictional instants (1950-05-15, 2000-01-01 and 2024-03-20 at 12:00 UT)
were combined with five sites (Rome, Melbourne, Quito, the equator at 0° E,
and Tromsø): **15 charts**, with the backend's default 15 requested bodies and
five house systems. Southern inversion was disabled in this comparison; the
separate backend smoke test exercises that option. The facade used the local
classic DE441, separately supplied Chiron SPK, and house methods from
`astrological-functions` branch `feat/houses-reference-validation-20261116`
commit `5b4df7dce4e00fa556fc22500e08295c723ec9f5`. It used the default UT
policy, not a loaded measured-Delta-T table.

The unchanged backend asks for `SEFLG_SWIEPH`. All **225 primary reference
calculations** reported Swiss ephemeris data in their effective flags; the
experimental facade explicitly maps that request to its configured JPL data.
This compares the current backend's source with the candidate source, not two
engines using an identical kernel. The Chiron orbit and model choices differ.
The source fingerprints, all reference fields and physical accuracy have not
been independently approved. No thresholds were approved before this run.

## Observations

All 15 backend calls returned without an exception. This alone is not proof of
complete results: the backend can discard individual calculation errors.

- Across the selected bodies/dates/sites, the largest geocentric longitude
  difference was **0.000266812°** (Moon), and the largest heliocentric
  longitude difference was **0.000234599°** (Mercury). The largest right
  ascension difference was **0.000272623°** (Moon). These are observations for
  the stated sources and times, not accepted error limits.
- For Vehlow, Placidus, Koch and Porphyry, compared cusps differed by at most
  **0.000000956°**. The reference returned an error for Placidus and Koch at
  the three Tromsø dates, while the candidate returned Porphyry fallback
  values, as previously decided. The reference returned Regiomontanus values
  at Tromsø that differ by approximately **180°** for some cusps; this known
  polar behavior is a deliberate unresolved compatibility difference, not
  evidence that either convention has been scientifically approved.
- The candidate's Placidus *planetary position* was unavailable in **six**
  circumpolar calls across the Tromsø cases. This is separate from producing
  fallback house cusps: the semi-arc method has no distinct rising/setting
  pair for these directions. The facade now returns the explicit non-enumerable
  error code `E_POSITION_UNAVAILABLE`, rather than an unspecified error code.
  The unchanged backend subtracts from `housePosition` and then uses a
  truthiness check; it silently omits these six positions. It also omits Chiron
  house positions by design in all 15 charts. The backend's truthiness check
  can also omit a valid zero position; that is a source-code finding, not an
  observed zero-position test in this matrix.

Default `swe_houses` results now expose `status`, `warnings` and `timeSource`
as **non-enumerable** properties, preserving the nine enumerated keys expected
by the legacy backend and its southern-inversion loop. Direct callers can see
the fallback warning. **JSON serialization omits these properties**, and the
unchanged backend does not yet surface them to product users. Enabling
enumerable provenance directly is not a safe drop-in for its existing loop.
An explicit backend integration is still needed to propagate fallback and
unavailable-position warnings in serialized responses.

## Reproduction and limits

`bindings/node/scripts/audit-chart-backend.mjs` runs this exact diagnostic on
caller-supplied local files and trusted modules. It prints per-field aggregate
maxima, sample counts, missing values, synthetic worst-case labels, effective
reference source counts and facade house-position error counts; no raw
reference rows are written. Its exit code detects whole-chart exceptions, not
all missing fields or scientifically unacceptable differences: inspect the
report and apply separately approved acceptance criteria.

```powershell
node bindings/node/scripts/audit-chart-backend.mjs <de441.eph> <chiron.bsp> <house-src> <local-swisseph-dir> <backend-func.js>
```

Replace placeholders with local paths; the script does not download files.
An aggregate report was retained outside Git and build output in
`VirgoStarsData/backend-validation/2026-11-16/chart-backend-diagnostic-v3.json`;
its SHA-256 is
`1aed951619c3d32f0e7d19bc6859e3218251741e0fd7eee771f5e83dffa9e7c4`.
A subsequent rerun regenerated the aggregate report byte-for-byte.
This small deterministic matrix is not a population study, independent JPL
validation, CI run, staging verification, migration or production rollout.
