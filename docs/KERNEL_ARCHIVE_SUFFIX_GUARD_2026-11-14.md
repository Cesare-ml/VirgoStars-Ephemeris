# Tracked kernel archive suffixes — 2026-11-14

Modification record under CPAL-1.0 §3.3. This focused change extends the existing repository data-distribution preflight; the CPAL-1.0 source header and VirgoStars attribution are unchanged.

## Motivation and behavior

`scripts/check_repository_data_hygiene.py` rejects tracked NASA/JPL and NAIF kernel filenames, even when a user deliberately forces an otherwise ignored file into the Git index. Previously, `is_kernel_path` stripped at most one suffix from `.gz`, `.xz`, `.bz2`, `.zst`, and `.zip`. A filename such as `de441.eph.tar.gz` or `DE440.BSP.zip.xz` therefore bypassed the **filename** screen; compressed bytes also lack the uncompressed SPK magic header.

The checker now repeatedly removes recognized archive suffixes, including standard `.tar` and tarball aliases `.tgz`, `.tbz2`, `.txz`, and `.tzst`, before comparing the remaining filename with the existing kernel-extension denylist. Ordinary archives and documentation such as `report.tar.gz`, `reference.csv.zip` and `model.bsp.notes.gz` remain permitted. Case-insensitive handling, raw magic-byte checks, Git-index inspection and symlink behavior remain unchanged.

This is a conservative **repository distribution guard**, not a scan inside generic archives or proof of rights to redistribute a kernel. It does not alter astronomical calculations, ephemeris values, supported formats, numerical units, time scales, reference frames or observer geometry.

## Verification

No external kernel is downloaded or committed. Synthetic unit tests cover nested compression suffixes, common tarball aliases, allowed documentation names and a Git-index regression for a forced tracked `.BSP.TAR.GZ` file without SPK magic bytes.

Run:

```sh
python3 -m unittest discover -s scripts/tests -p 'test_repository_data_hygiene.py' -v
python3 scripts/check_repository_data_hygiene.py
```

The author's environment can execute Python and isolated classification checks, but did not have an authenticated repository checkout. Run the exact repository test suite via read-only CI on the PR head before merging. This does **not** constitute NASA/JPL, NAIF or Horizons numerical validation.
