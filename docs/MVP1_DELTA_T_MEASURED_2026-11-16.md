# MVP 1 — measured ΔT for UT inputs (2026-11-16)

**Status: Locally tested. Data supplied by the caller; none bundled.** This closes
the largest known time-scale limitation of the UT entry (`Engine.calcUt`): the
earlier policy took UT1 = UTC after 1972 and used smoothed polynomial fits before
1972, which left the Moon off by up to 0.4″–0.6″ against the reference.

## What was added

- Core `delta_t_table`: a strict reader of three public US Naval Observatory files
  (`historic_deltat.data`, `deltat.data`, `deltat.preds`) that the caller provides as
  text, and a ΔT = TT − UT1 lookup. **No data is bundled and nothing is downloaded.**
  Sources and precedence: historic rows for years < 1973.0, the monthly file from
  1973-02-01, predictions only after the last observation. Linear interpolation in
  UT days; **no extrapolation** (before the first or after the last point the query
  fails). Each value carries its kind (`Measured`/`Predicted`) and its 1σ
  uncertainty when the source gives one. Malformed lines, non-increasing dates and
  gaps are errors with the file and line.
- Core `civil_apparent::jd_ut_to_tdb_with_table` and the time source
  `DeltaTTable { delta_t_seconds, uncertainty_seconds, predicted }`.
- Node `engine.loadDeltaT(deltatData, historicDeltat, deltatPreds?)`: afterwards
  `calcUt` converts UT with the table (from 1900-01-01 UT, the start of the
  precession-nutation model, to the last prediction, 2033-10-01), and
  `timeSource` reads `DELTA_T_TABLE:MEASURED|PREDICTED:<seconds>:<1σ|unknown>`. A
  failed load keeps the previous table; engines keep independent tables. Without a
  table the earlier policy is unchanged (and stops at 2027-06-28).

## Data

The maintainer authorized downloading the three files on 2026-11-16 (13,545, 30,345
and 1,483 bytes from `maia.usno.navy.mil/ser7/`). They are kept outside the
repository; URLs, sizes and SHA-256 are in
`docs/evidence/usno-deltat-source-provenance-2026-11-16.json`. Their reuse terms were
not reviewed, so they are not committed. The IERS C04 daily series was not
downloaded; monthly ΔT is enough for the Moon's 0.55″/s to stay within ±0.02″.

## Evidence

- Core: 4 synthetic-table tests (precedence, interpolation, flags, rejection,
  no extrapolation) and, with the genuine files, a check that the UT1−UTC implied by
  the table stays within ±0.9 s of UTC over **640 monthly points (1973–2026)** against
  the IERS leap-second table, an independent consistency test of the table and of
  the time chain.
- Node: 5 tests (policy without table, table source label, failed-load rollback and
  `E_DELTAT_FILE`, engine independence, genuine files). Full suite with kernel,
  catalog, USNO files and the real `swisseph` package: **35 of 35 pass**; without data
  11 pass and 24 skip.
- Against the reference (installed `swisseph` 0.5.17, JPL mode, local only; receipt
  `docs/evidence/mvp1-delta-t-measured-local-reference-receipt-2026-11-16.json`),
  127 UT dates, maximum longitude difference by era:

  | Era | Moon | Other bodies |
  |---|---|---|
  | 1900–1930 | 0.66″ | 0.08″ |
  | 1930–1972 | 0.28″ | 0.03″ |
  | **1972–2018** | **0.02″** (was 0.39″) | 0.002″ |
  | 2018–2027 | 1.8″ | 0.21″ |

## Why the early and the recent eras still differ

The reference's own ΔT agrees with the measured table to within 0.06 s from 1941 to
2018 (checked at 9 dates; 0.17 s at 1926, 0.31 s at 1912, 0.7 s at 1900).
**Outside that window the two ΔT series differ, and for the recent years the
measured one is the consistent one:**

- After about 2018 the reference's ΔT keeps growing (71.9 s in 2026) while the
  measured values stay near 69.1 s, the figure that also follows from the IERS leap
  seconds. Judged by the Moon's position the reference's ΔT is about 0.6 s too large
  in 2022, 2.1 s in mid-2024 and 2.8 s in 2026, hence the 1.8″ on the Moon. Positions computed with the
  reference ΔT for recent births are therefore off by about 1–2″ on the Moon (about
  0.0005°), negligible for a chart but not zero; the new engine is closer to the
  physical value.
- In 1900–1930 the USNO historical table and the reference differ by 0.2–0.7 s
  (the reference gives −1.7 s at 1900.0, the table −2.4 s, the literature fits
  about −2.7 s). Which one is right there is beyond what this comparison decides;
  the table carries ±0.06 s errors there.

## Not done

- Daily UT1−UTC from IERS C04 (would remove the monthly-interpolation error, below
  0.02″ for the Moon).
- ΔT outside 1657–2033 and before 1900 for chart use (the orientation models start
  in 1900).
- A civil-time entry that uses the table before 1972 (the civil path still uses the
  polynomial there; only the UT-Julian-date entry uses the table).
- House angles use the declared policy; ΔT matters there below 2e-6″ per second.
