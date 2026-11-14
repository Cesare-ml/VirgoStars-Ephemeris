# External kernel distribution preflight — 2026-11-14

## CPAL-1.0 modification record

This change adds a **read-only repository distribution guard** and deterministic unit tests in `scripts/`, and enables the guard in the existing Linux Rust development CI workflow. Original Developer: Cesare-ml (VirgoStars project); source notices and the CPAL-1.0 required VirgoStars attribution remain unchanged.

## Purpose

JPL/NAIF kernels and Astrodienst/Swiss ephemeris data are external materials **not distributed under the project's CPAL-1.0 source-code license**. Even though `.gitignore` excludes common kernel extensions, `git add -f` can stage them and `.gitignore` does not protect files renamed to an innocuous extension.

The new `scripts/check_repository_data_hygiene.py` reads the Git **index** using `git ls-files --cached -z` and rejects:

- Tracked filenames ending in `.bsp`, `.eph`, `.se1`, `.bpc`, `.tpc`, `.tls`, `.tf` or `.tsc`, case-insensitively, including immediate `.gz`, `.xz`, `.bz2`, `.zst` and `.zip` suffixes.
- Regular tracked files whose first bytes identify a `DAF/SPK` NAIF kernel or classic `JPL Planetary Ephemeris DE` payload, even if the filename was disguised.

Symlinks and Git submodules are never dereferenced. The script only reads at most 64 bytes of each other tracked regular file; it does not download or analyze scientific kernel content, contact NASA/JPL, redistribute data or rewrite the index.

This check **does not prove that a repository contains no copyrighted, proprietary, compressed-with-an-unrecognized-name, or otherwise restricted data**. It is a narrow guard for common accidental kernel inclusions, not license clearance or real astronomical validation.

## Run locally

```sh
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
python3 scripts/check_repository_data_hygiene.py --repo .
```

The tests use only Python's standard library and temporary Git repositories. They exercise forced `git add -f`, case-insensitive extensions, compressed suffixes, disguised binary signatures and intentionally untracked local files. Expected script exit codes are 0 for clean, 1 for a detected tracked payload and 2 for inspection/Git errors.

The existing `Rust development CI` Linux job runs these tests and the index check **before** installing the Rust toolchain. Changes under `scripts/**` now trigger that workflow on `dev/ephemeris` pull requests and pushes. This is read-only validation with no new CI credentials, release jobs or scientific data downloads.
