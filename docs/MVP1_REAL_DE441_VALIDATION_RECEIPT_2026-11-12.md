# Reproducible local real-DE441 scientific validation receipt

**Date:** 2026-11-12  
**Status:** experimental opt-in runner; **no authentic DE441 file has been executed in GitHub CI or this PR**.

This branch is stacked directly on [draft #122](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/122), which introduced four optional **`#[ignore]`** real classic-DE441 tests split across two Rust test suites. Those tests compare geocentric one-way-light-time (**LT**) geometry against existing independent NASA/JPL Horizons numerical reference data, and check three signed aberrated-output rates by full-pipeline finite differences. `scripts/run-real-de441-validation.ps1` makes those tests **auditable and easy to execute locally on Windows 11 using PowerShell 7**, without requiring `--ignored` flag knowledge, copying the kernel, or allowing skipped tests to be treated as passing.

## Run against your locally supplied authentic classic DE441

The file must be an **actual classic binary `de441.eph`**, not compressed Astrodienst `.se1`, NAIF SPK `.bsp`, an arbitrary renamed large file or a 9 KB fictional test fixture. Obtain or identify it independently and check its redistribution and provenance terms. The runner refuses missing files, files smaller than **1,000,000,000 bytes** and SPK/DAF-magic headers; the existing Rust reader still validates the classic format and ephemeris series.

At a terminal in the checked-out **feature branch containing this script and draft #122's test file**, with **PowerShell 7, Rust and Cargo installed**:

```powershell
pwsh -NoProfile -File ./scripts/run-real-de441-validation.ps1 `
  -De441Path "D:\Astronomy\de441.eph" `
  -OutputRoot "D:\VirgoStars-Scientific-Reports"
```

`-OutputRoot` is optional: the default is `$HOME/VirgoStars-Validation`. Every run creates its own unique directory, e.g. `20261112T205500Z-1234abcd`. **Nothing is sent to GitHub, an external site, or an API.** The actual file is only opened/read locally by SHA256 and Rust tests, never copied into the report. Report output contains only the **kernel filename, byte size and SHA-256**, not the file contents or absolute kernel path. Avoid uploading the raw kernel to GitHub or attaching it to an issue. If a review needs the report, share `receipt.json` and the generated text logs, after checking them for your own privacy requirements.

## Test coverage and independent reference boundaries

The runner explicitly invokes each complete suite, **not** `cargo test --workspace -- --ignored`:

```sh
cargo test --workspace --locked --test light_time_horizons -- --ignored --nocapture
cargo test --workspace --locked --test aberrated_real_de441 -- --ignored --nocapture
```

The first suite executes **two pre-existing real-file tests**:

1. `light_time_states_match_horizons_for_solar_system_targets`: checks the Earth-geocentric one-way LT positions, Horizons instantaneous velocity convention and full LT derivative consistency, comparing Moon/Venus/Mars barycenter/Jupiter barycenter at **TDB JD 2461354.5**, ICRF, `VEC_CORR=LT`, `CENTER=500@399` (the reference was recorded 2026-11-10).
2. `real_kernel_covers_common_bodies_and_rejects_missing_emission_history`: evaluates multiple bodies across the documented reference epochs and checks that missing target emission history is **rejected**, not silently extrapolated.

The second suite executes **two draft #122** tests:

3. `real_de441_aberrated_pipeline_recovers_horizons_light_time_geometry`: inverse-applies the same Rust special-relativistic aberration boost to recover **LT-only ICRF XYZ** and compares it with the independently recorded Horizons **LT reference**. This does **not independently verify** the boost law itself, independent `LT+S` values, gravitational deflection or a moving ecliptic-of-date.
4. `real_de441_aberrated_batch_rates_follow_neighboring_full_pipeline_states`: compares the three output rates against centered finite differences of the same full DE441 pipeline at TDB ±600 s; this is internal rate-consistency evidence **using real input data**, not an independently computed JPL apparent-velocity reference.

Numerical thresholds in the original tests have not been scientifically calibrated against separate, real apparent-coordinate references. A passing report is labelled **`PASS_LIMITED_LT_REFERENCE_AND_INTERNAL_RATES`**, not full apparent-coordinate, Swiss `swe_calc_ut`, topocentric, IAU precession/nutation or other parity.

## Fail-closed evidence contract

The script refuses to mark a gate as successful based solely on `cargo` exiting zero: it additionally requires the **exact two expected named test cases** to appear as `... ok` and a Rust test summary showing **2 passed / 0 failed / 0 ignored**. Thus CI builds where the ignored tests were skipped, or suites run with an incorrect filter, cannot be presented as real-kernel validation.

Its report directory contains:

- `receipt.json` — schema 1, UTC execution timestamp, source Git commit when available, filename/size/streaming SHA-256, Cargo and rustc versions, pinned provider/query correction settings, expected targets, per-suite **command, status and exit code**, and a declared scientific interpretation limit.
- `light_time_horizons.log` — full stdout/stderr of the original independent-LT-reference suite.
- `aberrated_real_de441.log` — full stdout/stderr of the new reconstructed-LT and derivative tests.

Every failure produces **a nonzero exit code and `status=FAIL`** in the receipt; if validation stops before tests, it records `NOT_RUN` only until the first error and never writes a misleading `PASS`. Logs that are not executed are not invented. The runner restores any pre-existing process `DE441_PATH` variable on completion, and does not overwrite existing reports.

**Important verification limitation:** the 1 GB size check and SHA256 fingerprint provide a minimum format guard and **identity** for a supplied binary, not proof the file truly originated from NASA/JPL. Verify the hash against a trusted distributor separately. Do not interpret a simulated/selftest pass as evidence of real-file scientific accuracy.

## CI selftest without an authentic DE441 file

```powershell
pwsh -NoProfile -File ./scripts/run-real-de441-validation.ps1 -SelfTest
```

The selftest checks that the validation preflight rejects a missing file and a fictional **9 KB** kernel. It also uses **invented log lines** to verify that the receipt parser rejects skipped tests, partial test executions, failed assertions and nonzero Cargo exit codes, while accepting precisely two successful cases. It **does not invoke Cargo's real-file tests**, calculate real planets, hash gigabytes of data or claim independent accuracy.

The existing authorized **read-only Windows GitHub CI job** runs only this `-SelfTest`; ordinary Linux and macOS Rust tests remain unchanged. The CI still has `permissions: contents: read`, no secret, no external download, no privileged workflow, no package publishing and no release. Because the real scientific PR is stacked, a temporary validation-only draft PR to `dev/ephemeris` may trigger these already-approved checks; that temporary PR must be **closed without merging**.

## Modifications, licensing and dependency review

**CPAL-1.0 modification record, 2026-11-12:** original PowerShell script `scripts/run-real-de441-validation.ps1` with the project's Exhibit A source notice, one focused read-only CI Windows preflight step and trigger path, plus this English documentation. The existing `crates/ephemeris-core/tests/*.rs` mathematical algorithms, `LICENSE`, `NOTICE` and any data/binaries remain unchanged. No NASA, NAIF, Swiss or proprietary code is incorporated, and no real `.eph` file is bundled.

Required attribution for applicable users: **Powered by VirgoStars** — https://www.virgostars.it/.

This PR is draft above the independent real-kernel scientific gate [#122](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/122), itself stacked on #118/#114/#107/#77 with civil-time #96/#110 dependency review outstanding. It does not merge into `dev/ephemeris` or `main`, change production, add third-party file redistribution, or assert that the complete apparent-coordinate pipeline is validated.
