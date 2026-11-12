# MVP-1: actual CLI exporter → comparator round-trip regression (offline)

**Date:** 2026-11-12. **Status:** experimental test workflow, not real JPL numerical validation.

## Why this check is necessary

Unit tests for [draft #126](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/126) separately checked its CSV writer and command-line parsing. Unit tests for [draft #132](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/132) separately checked its strict CSV comparison parser, tolerance rules and labels. Neither set alone established that a **real invocation of the full exporter process** actually produced a CSV which the comparator process could consume and audit.

`scripts/test-aberrated-csv-end-to-end.ps1` is a **PowerShell 7** script that runs both **real compiled Rust examples** via `cargo run --quiet --locked`. It writes its data only under a unique automatically cleaned temporary directory. It does not change the Rust astronomy core, download anything, bundle ephemeris data, invoke GitHub Actions APIs or use network resources. The script's dummy binary is only ~9 KB and is generated anew.

## What is exercised

1. Generate a tiny **fictional** classic-DE441 binary using the exact synthetic record convention from `tests/civil_aberrated_batch.rs`: quadratic Mercury, Earth-Moon barycenter and geocentric Moon coefficients, including nonzero analytic Earth acceleration. This format-level fixture is **not** an official NASA/JPL kernel, astronomical reference, or proof that any resulting coordinates match the Solar System.
2. Execute `aberrated_csv <synthetic.eph> 2026-11-10T13:30:00+01:00 Moon Mercury Moon` and again with the equivalent `2026-11-10T12:30:00Z`. This is the **actual CLI path**, including civil UTC-offset parsing, approximate TDB conversion, classic DE441 file I/O, target iteration, light-time solution, annual special-relativistic aberration and six-field unit formatting.
3. Verify both CSVs match **byte-for-byte on their text lines**, contain a header plus three rows, and retain duplicate Moon targets in order. Re-run the exporter with an explicit `tdb:<JD>` read from its own output and assert exactly identical CSV. Also request an ambiguous unsupported physical `Mars` body and check that the exporter fails **without partial CSV output**.
4. Invoke the real `compare_aberrated_csv` with **zero tolerances** and confirm that it accepts those two same-instant outputs, emits a complete four-line report and reports no failure rows.
5. Deliberately alter **one signed angular speed** by +0.25 degrees/reception TDB day in one row, holding metadata unchanged; confirm the comparator exits nonzero and produces a complete report marking that row as outside tolerance. This is an observed numerical mismatch, not a label mismatch.
6. Deliberately replace the reference frame label with `ecliptic_of_date`, retaining everything else; confirm that the comparator exits nonzero **before emitting any residual report**. This prevents the false impression that comparing different frames provides astronomical validation.

The script stops with an exception if any expectation fails. All paths are local and temporary; it doesn't need a full 2.8 GB DE441 file or environmental `DE441_PATH`.

## Reproduce on Windows / CI

At the repo root on **PowerShell 7** with Rust/Cargo installed:

```powershell
pwsh -NoProfile -File ./scripts/test-aberrated-csv-end-to-end.ps1
```

The existing authorized **read-only** `.github/workflows/rust-dev.yml` has been extended by a narrowly scoped step in its **Windows native Rust tests** job:

```yaml
- name: Smoke-test offline CSV exporter and comparator together
  shell: pwsh
  run: ./scripts/test-aberrated-csv-end-to-end.ps1
```

The normal CI also runs Linux Rust fmt, debug/release workspace tests, strict Clippy, Windows native tests and macOS native tests. The script is included in PR/push workflow path filters so later corrections to its contents are tested. The workflow retains `permissions: contents: read`, with **no additional token permissions, secrets, deployments, external file downloads or package publishing**. A temporary CI-only draft into `dev/ephemeris` is used for stacked verification and must be **closed without merge**.

## What passing does and does not prove

This is **software interoperability and error-detection evidence**: the exported, explicitly time-scaled **fixed ICRF equatorial** geocentric, one-way-light-time plus annual-aberration six-field values in degrees/AU and rates per reception TDB day can be compared by the actual CLI with zero tolerance, and the CLI detects tampered speed values and bad frame labels. It preserves the exact Rust `Body` IDs, including Moon duplicates.

It does **not** prove:
- Real classic DE441 kernel accuracy; independent JPL/Horizons/Solar System coordinate parity; or compatibility with Swiss `swe_calc_ut`.
- An independent reference for the annual-aberration boost law or proper apparent **true ecliptic of date** coordinates.
- Precession, nutation, gravitational light deflection/Shapiro delay, topocentric effects, physical Mars–Pluto planetary centers, precise historical UTC policy or real ephemeris redistribution rights.
- Performance improvement or production readiness. The dummy coefficient values are deliberately unrealistic.

The optional **real-kernel** tests live separately in [draft #122](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/122); those are deliberately skipped without a locally provided, legally acquired classic DE441 file.

## Safe provenance and integration

**CPAL-1.0 change record (2026-11-12):** original PowerShell test script with a preserved CPAL Exhibit A notice, one scoped existing read-only CI workflow step, path-filter registration and this English document. No third-party data, code or new dependencies are committed; `LICENSE` and `NOTICE` remain untouched. Applicable attribution remains **Powered by VirgoStars** — https://www.virgostars.it/.

This draft stacks on **#132**, which stacks on #126 → #118 and the existing #114/#107/#96 dependent scientific pipeline. None of the upstream drafts should be implicitly considered merged, and no protected branch changes or production deployments are authorized by a passing synthetic smoke test.
