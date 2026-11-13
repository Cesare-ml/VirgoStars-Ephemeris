# E1 SPK reference matrix breadth audit (synthetic-only validation helper)

**2026-11-13 — experimental offline test-preparation utility.** This does **not** establish genuine JPL/SPICE numerical accuracy or complete milestone E1.

## Why this gate exists

Issue [#3](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/3) requires multiple target/observer states and at least three epochs before scientific acceptance. A validator accepting a single well-formed row could otherwise return success even though the reference set is far too narrow. The standalone [`scripts/audit_spk_e1_matrix.py`](../scripts/audit_spk_e1_matrix.py) checks **only CSV structure and case breadth**, independent of unmerged NAIF/SPK implementation PRs.

The audit does *not* calculate astronomical vectors or authenticate any claimed `provider=NAIF_SPICE`, SHA-256, JPL kernel, toolkit version, or source script. Rust remains the only VirgoStars astronomical engine. Run the separate canonical Rust/NAIF comparators and verify real-kernel hashes/provenance before making any scientific claims.

## Strict minimum matrix policy

A candidate reference must contain these **exact numeric NAIF target/center pairs**:

- Sun center relative to Earth center: `10:399`.
- Moon relative to Earth center: `301:399`.
- Earth center relative to solar-system barycenter: `399:0`.
- At least **two physically distinct planetary systems** from NAIF barycenter IDs `1,2,4,5,6,7,8,9` or planet-center IDs `199,299,499,599,699,799,899,999`, each relative to Earth center `399` or SSB `0`. **A planet center and its corresponding system barycenter count as the *same* planet**, e.g. `4` (Mars-system barycenter) and `499` (Mars center) are not two distinct systems; they are also not interchangeable vectors.

Each of those five selected target/center pairs needs **at least three distinct common ET values** (TDB seconds past J2000); shared epochs make a reproducible cross-body matrix. The audit checks CSV metadata for `provider=NAIF_SPICE`, `method=spkezr`, `time_scale=TDB`, `frame=J2000`, `corrections=NONE`, `units=KM-S`, required provenance strings and declared 64-hex ordered kernel digests. It rejects nonfinite ET/components, non-integer NAIF IDs, same-body pairs, duplicates (including signed zero), invalid columns and malformed metadata.

**No implicit defaults:** the auditor does not invent observation times, convert UTC to ET, assert that any NAIF ID exists in a given kernel, enforce numerical tolerances or infer support for planetary center versus barycenter. A `PASS` is **only** a case-matrix breadth check. Check SPK actual segment coverage before selecting epochs; verify record/segment boundaries, different byte orders, light-time/frame corrections and Horizons solution identity separately. The minimal breadth gate is **necessary but not sufficient** for E1 and is deliberately stricter about shared epochs than issue #3's simple three-epoch wording.

## Offline usage

```bash
python3 scripts/audit_spk_e1_matrix.py --reference /path/to/reviewed-spk-reference.csv
python3 -m unittest discover -s scripts/tests -p 'test_spk_e1_matrix.py'
```

On Windows use `python` in place of `python3`. Supply a **locally obtained** reference CSV with the canonical header:

```text
et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s
```

Metadata in the format already used by the experimental NAIF/SPICE reference generator is required before this header. The command returns **0** only for a structurally valid sufficiently broad matrix; **1** for a valid but incomplete matrix; **2** for malformed/unreadable CSV. It reports the selected two planetary pairs and shared epochs when the breadth check passes.

The included Python standard-library `unittest` suite uses **fabricated numeric rows**, not real NAIF output, and is run by the existing read-only Rust development CI on Ubuntu, Windows and macOS. Follow-up regressions prevent counting the same physical planet twice via its distinct NAIF center and barycenter IDs, while retaining support for these two separate reference-vector definitions. No external kernels, network calls, downloads, dependencies, or datasets are required.

## Scientific review checklist before calling E1 validated

1. Record official kernel URL, size, SHA-256, legal provenance and supported descriptor coverage. Do not redistribute kernels inside this repo.
2. Obtain actual independent NAIF/SPICE `J2000/NONE` km/km-s states at the approved TDB/ET epochs for the audited body matrix.
3. Verify exact kernel load order, reference generator and toolkit versions, input files, and separately approved position/velocity tolerances.
4. Run canonical Rust residual comparator against the real reference with signed XYZ/VXYZ differences and provide its evidence receipt. Independently check the matching JPL Horizons ephemeris solution and required boundary cases.
5. Keep `E1=IMPLEMENTED_UNVERIFIED` until the complete real-data evidence has been reviewed. A passing synthetic CI run or this script's PASS is never scientific parity evidence.

## CPAL-1.0 modification record

2026-11-13 (follow-up): corrected planetary-system identity when selecting independent test targets: planet center and system barycenter IDs now map to the same physical planet for coverage counting. Added regression tests without changing astronomical calculations.

2026-11-13: Added original `scripts/audit_spk_e1_matrix.py`, synthetic regressions in `scripts/tests/test_spk_e1_matrix.py`, and safe Python tests in the existing three-platform development CI. Original source stays under the project's [CPAL-1.0 license](../LICENSE), [NOTICE](../NOTICE), and mandatory VirgoStars attribution; no third-party software, datasets, production changes, publication or release.
