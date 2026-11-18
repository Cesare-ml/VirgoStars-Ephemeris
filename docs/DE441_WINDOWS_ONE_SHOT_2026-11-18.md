# VirgoStars Mobile DE441 — one-shot Windows validation and pack build

**Status:** experimental source tooling only. The complete 10-pack set has
**not** yet been generated or tested using a real JPL DE441 kernel. No public
download, Android deployment, or permission to redistribute datasets is
implied. This guide applies to the **classic little-endian** DE441 binary,
not NAIF SPK `.bsp`.

## Prerequisites

1. Windows 11, sufficient disk space (recommend **at least 5 GB free** where
   the original 2.6 GB-class file will be downloaded, plus build space).
2. Python **3.10+** and Rust stable with `cargo`, `rustfmt` and `clippy`
   available in the PowerShell session. The script checks the tools and stops
   on error. It does **not** silently install or update programs.
3. A checkout of the
   [VirgoStars-Ephemeris experimental feature PR](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/601)
   (branch `feat/de441-classic-slice-export-20261118`).
4. Write permissions in an **output directory outside any Git repository**.
   Existing packs, manifest files and reports will never be overwritten.

## Run once — all 10 packs

From the repository root in **Windows PowerShell**, execute:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run_de441_mobile_windows.ps1 -OutputDirectory "D:\VirgoStars-DE441-Mobile" -DownloadIfMissing
```

The optional `-DownloadIfMissing` flag obtains the original ~2.6 GB
`linux_m13000p17000.441` file from the user-specified official
[JPL planetary ephemerides archive](https://ssd.jpl.nasa.gov/ftp/eph/planets/Linux/de441/linux_m13000p17000.441),
to a separate `source` folder inside the output directory. The file
download is initiated only **after** available local tests pass. It is
**never committed** to the repository. A partial download is deliberately
left named `.download-partial` for manual inspection, not silently replaced.
No downloaded source file is ever uploaded by this script.

If a classic DE441 file is **already present**, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run_de441_mobile_windows.ps1 -SourcePath "D:\Data\de441.eph" -OutputDirectory "D:\VirgoStars-DE441-Mobile"
```

Append `-BaseOnly` to generate **just 1600–2100** while retaining the
complete 10-range plan in the manifest. For the entire set use the default
(no `-BaseOnly`). A future all-pack execution after a base-only run must
use a **new** output directory, to avoid overwriting the existing manifest.

For reproducible processing of an independently verified input, append
`-ExpectedSourceSha256 "<64-character hex digest>"`. Without a preapproved
digest the tool computes and records the real source SHA-256 but does not
claim to authenticate NASA/JPL provenance.

## What the one-shot command executes

1. Rust quality gates:
   `cargo fmt --all -- --check`, `cargo test --workspace`,
   `cargo clippy --workspace --all-targets -- -D warnings`.
2. Python regression tests and repository data-hygiene guard.
3. Source download (only when requested/missing); Rust export for each
   selected year interval; SHA-256 of source and every output; exact header
   and coefficient-record coverage checks.
4. **Rust source/slice numerical parity:** at the start, halfway and 7/8
   of *every coefficient record*, compare geometric ICRF positions (km)
   and velocities (km/TDB-day) for 14 pairs, including planets, Earth,
   Moon and the Sun. The final shared pack endpoint is tested only when
   the source ends there, avoiding distinct adjacent Chebyshev polynomials.
   Any mismatch fails the pipeline before the manifest is published.
5. Rehash installed files and write a JSON manifest, a human-readable
   byte/MB/MiB report and PowerShell log.

This is **source/slice identity**, not a mathematically independent JPL
reference comparison. An independent SPICE/Horizons validation, provenance
review, signature/manifest approval, Android download manager and license
review remain separate release gates.

## Expected output

```text
D:\VirgoStars-DE441-Mobile\
  source\linux_m13000p17000.441     # only if DownloadIfMissing is used
  de441-run-YYYYMMDD-HHMMSS.log
  packs\
    de441-core.eph
    de441-history-0000-0199.eph
    ...
    de441-future-2101-2300.eph
    de441-mobile-manifest-v1.json
    de441-mobile-report.txt
```

Each `built` manifest entry includes the exact stored/requested Julian Date
TDB bounds, byte size, file SHA-256, record count and the Rust comparison
sample count. `not_built` entries provide only a planned range, not a
downloadable artifact. Manifest release status remains `review_pending`.

**After running**, share only
`packs/de441-mobile-report.txt` and the transcript log for review.
Do **not** upload the original 2.6 GB file or generated `.eph` packages
to GitHub before the dataset rights and provenance checks.

## Troubleshooting

- **No Rust/Python:** install the toolchains yourself, reopen PowerShell,
  and run the same command again. No automatic software installation.
- **Cargo fmt/test/clippy fails:** stop; fix the feature branch before
  generating data. The script intentionally fails closed.
- **Download interrupted:** the partial file is preserved under `source`;
  inspect its size before deciding whether to remove it and retry.
- **Build or parity fails:** intermediate `.eph` files may remain. Nothing
  is silently deleted and no approved manifest is published; retry with
  a different output directory after diagnosing the error.
- **Missing date range in the Android app:** this pipeline only creates
  local files. It does not yet implement mobile Settings, downloads,
  signatures, or installation/removal of packs.

## Modification and licensing record — 2026-11-18

Original VirgoStars-Ephemeris CLI, regression/manifest helper, and Windows
orchestrator. The original source code is governed by CPAL-1.0 with the
repository's `LICENSE` and `NOTICE`; JPL binary data is external and
**not relicensed** under CPAL. The tool does not include any JPL kernel.
