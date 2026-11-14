# Atomic publication of independent SPK reference CSV

**Modification: 2026-11-14.** The independent validation-only script
`scripts/spk_naif_reference.py` now publishes generated CSV files atomically.
This change does **not** alter NAIF/SPICE calculations, reference metadata,
external kernel bytes, Rust interpolation, scientific thresholds or consumer
APIs.

## Correctness problem

Previously, `generate` checked `output.exists()` and later used
`Path.write_text()`. Between these two operations, another writer could
create the same output file. The late write would silently overwrite that
file even **without** `--overwrite`. A disk failure midway through
`write_text` could also leave a partial final CSV with an official-looking
name, potentially misleading a subsequent operator.

## Publication contract

The new `write_reference_atomic` function stages the entire UTF-8 CSV in a
unique temporary file **in the output directory**, flushes and `fsync`s
its contents, and only then publishes the final name.

- Default (without `--overwrite`): a same-directory **hard link** is
  created atomically. If the destination appeared at any point, publication
  fails without overwriting the competing file; temporary data is removed.
- Explicit `--overwrite`: uses **`os.replace`** on the same filesystem
  so an existing name is replaced atomically with a complete CSV.
- A failure to stage, sync, link or replace fails **closed**; no fallback to
  non-atomic `Path.write_text` is permitted. Temporary paths are removed
  after both success and failure.
- Existing early collision checks are retained for fast feedback, but the
  final atomic publication is the **authoritative** race prevention.
- A platform/filesystem without same-directory hard-link support is not
  supported for exclusive publication and must return an I/O error.
  This is safer than silently allowing overwrite.

The independent reference includes the SHA-256 of the **exact source file**
used to generate it. Updating the generator source changes that SHA-256;
a CSV previously produced by a different script revision is expected to
fail its strict source-hash check unless verified using that original
revision. Do not rewrite provenance hashes to force old evidence through
a newer validator.

## Verification

Five additional deterministic Python unit tests under
`scripts/tests/test_spk_naif_reference.py` cover successful publication,
existing-file preservation, a deterministic simulated time-of-check/time-of-use
race, explicit overwrite, and simulated `fsync` failure with cleanup.
No installed SpiceyPy/NAIF Toolkit or network is needed for these tests.

```bash
python3 -m unittest discover -s scripts/tests -p 'test_spk_naif_reference.py'
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

Existing read-only Rust CI runs the Python tests on Linux, Windows and macOS.
The change must be reviewed for compatibility with outstanding **#164**, which
extends the **same script** with a JSON receipt and one-command Rust validation;
its additional output paths need separate atomic-publishing review.

## Scientific and licensing limits

An atomic CSV is **not** evidence that independent NAIF state vectors have
been compared against an official NASA/JPL kernel. E1 remains unverified
until a provenance-reviewed local kernel, actual NAIF/SPICE reference values,
documented TDB/ET/frame/center/correction settings, independently approved
residual tolerances and a real Rust-comparator receipt are examined.

No `.bsp`, `.eph`, Swiss code, Toolkit source, secrets or new runtime
dependencies are added; no kernel download, package release, production
migration or deployment. CPAL-1.0 Exhibit A and **Powered by VirgoStars**
requirements continue to apply.
