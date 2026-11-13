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

## Plan explicit NAIF/SPICE test cases

The same standalone utility can **plan** the minimum body/epoch matrix before generating NAIF vectors. The operator must supply *two physically distinct planetary systems* using their exact numeric NAIF target IDs, and **three or more explicit finite TDB/ET epochs** (seconds past J2000). No default planets, timestamps, UTC conversion, dataset loading, downloads, or scientific accuracy claims are involved.

```bash
python3 scripts/audit_spk_e1_matrix.py --plan \
  --planet 4 --planet 5 \
  --epoch 0 --epoch 86400 --epoch 172800
```

The 15 output lines use the exact `--case TARGET:CENTER:ET` syntax accepted by the experimental `scripts/spk_naif_reference.py generate/run` commands in unmerged drafts #162/#164. For example, the first and last printed lines are `--case 10:399:0` and `--case 5:399:172800`. Review them and pass them as repeated `--case` arguments; the planning command **does not execute** SPICE or Cargo. On Windows, use `python` and PowerShell backticks instead of bash line continuations, or issue a single-line command.

For planetary test pairs the planner uses **Earth center (399)** as observer, without silently substituting barycenters or changing a caller-supplied target ID. It rejects `--planet 4 --planet 499` as **one** Mars system, nonfinite/duplicate epochs including `-0` versus `0`, and an incomplete set of planets or epochs. Actual kernel support for chosen targets, frames, centers and ET values must be established from the relevant SPK descriptors before running NAIF. This planner sets up the minimum general-purpose E1 case matrix, **not** separate near-record/segment boundary tests required by issue #3.

## Machine-readable audit report (not a scientific receipt)

Use `--json` with `--reference` to emit one JSON document on stdout that
other local validation tools can consume. Unlike the independent NAIF/Rust
**numerical** receipt from draft #164, this JSON object reports only **E1
test-matrix breadth** and must never be interpreted as verified scientific
accuracy.

```bash
python3 scripts/audit_spk_e1_matrix.py --reference /path/to/spk-reference.csv --json > matrix-audit.json
```

A structurally valid and complete synthetic example produces fields such as:

```json
{
  "schema": "virgostars-e1-matrix-audit/v1",
  "status": "pass",
  "reference_sha256": "<actual SHA-256 of exact input CSV bytes>",
  "case_count": 15,
  "planet_pairs": [[4, 399], [5, 399]],
  "shared_et_tdb_seconds": ["0", "86400", "172800"],
  "issues": [],
  "scientific_accuracy_certified": false,
  "provenance_review_required": true
}
```

This is an **illustration**, not a real file fingerprint or NAIF result.
`reference_sha256` binds the report to the exact **raw CSV bytes**,
including line endings, rather than the normalized numeric representation.
The three TDB epoch strings preserve Python's 17-digit round-trip rendering.
Compare the byte hash with the source reference used by any separate Rust
comparison receipt. Neither a matching hash nor a `status=pass` authenticates
the claimed NAIF provider or vector values.

The command uses the **same exit status as text mode**: `0` on sufficient
matrix breadth, `1` on incomplete but valid input, `2` on malformed/missing
input. The JSON status is respectively `pass`, `incomplete`, or `invalid`.
Invalid input yields an `issues` list and the explicit
`scientific_accuracy_certified=false` flag, but does **not** invent a
`reference_sha256`. Consumers must require both exit code zero and
`status=pass`; the full astronomy/provenance gates must be checked separately.
`--json` and `--plan` cannot be combined.

## Optional cross-check with the NAIF/Rust numeric receipt (draft #164)

The receipt from draft [#164](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/164) is an **unmerged, experimental** JSON record of a separate Rust comparator run. When both reference CSV and receipt are available, the E1 auditor accepts an optional `--receipt` argument:

```bash
python3 scripts/audit_spk_e1_matrix.py \
  --reference /path/to/spk-reference.csv \
  --receipt /path/to/spk-comparison-receipt.json \
  --json
```

Before reporting matrix breadth, it checks the receipt's `reference_sha256` against the **actual CSV bytes**, `case_count` against the parsed number of rows, the declared `J2000/NONE/TDB seconds/km/km-s` contract, real-toolkit-version string equality between files, generator script SHA-256, and the **ordered** kernel names, sizes and SHA-256 declarations against the CSV. The receipt must explicitly declare `COMPARATOR_PASS_PROVENANCE_REVIEW_PENDING`, `scientific_accuracy_certified=false`, `provenance_review_required=true`, and positive finite numeric comparator thresholds. The auditor now validates the **complete canonical offline Rust command arguments**, not just its prefix: the `--` separator must be followed by the declared absolute reference CSV path, precisely serialized position/velocity thresholds, and the ordered absolute SPK kernel paths with filenames matching the manifest. The receipt's reference path must also match the actual `--reference` file supplied to the auditor. This catches stale-but-identical CSV copies, mismatched thresholds, renamed kernels and commands padded with extra arguments, without executing any command. The command does **not** open or rehash kernel binaries; that remains the responsibility of the independent preflight in drafts #162/#164. Missing or mismatched evidence exits **2** (`invalid`), even if the CSV contains a complete matrix. A matched receipt adds `"receipt_consistency_checked": true` to the JSON report; ordinary CSV-only reports include `false`.

This is an **evidence consistency cross-check, not authentication**. Both JSON and CSV can be fabricated together, the declared Rust command is not re-executed by this helper, and a passing CSV/receipt fingerprint does not establish independent NAIF provenance or JPL numerical parity. Do not treat the output as a scientific certificate. Genuine E1 acceptance still requires real local JPL kernels, independent reference vectors, approved tolerances, and a reviewed Rust numerical residual receipt. The optional check deliberately avoids importing #164 code or changing its unmerged generator and receipt writer.

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

2026-11-13 (full command trace follow-up): strengthened the offline receipt auditor to require exact Rust argv consistency (CSV path, serialized numeric tolerances and ordered kernel paths) and reject stale CSV evidence paths even when bytes match. Added four synthetic regression tests. This is evidence self-consistency only, not proof of a real NAIF/Rust invocation.

2026-11-13 (receipt cross-check follow-up): added optional `--receipt` correlation against the experimental NAIF/Rust comparator receipt in draft #164, with strict exact-byte CSV fingerprint, case count, scientific-contract, ordered-kernel metadata and non-certification flags. Nine synthetic regressions verify rejection of missing or incompatible evidence. No numerical comparator, toolkit dependency or third-party dataset was added.

2026-11-13 (reporting follow-up): added a machine-readable JSON breadth result with explicit non-certification markers and SHA-256 of the exact local reference CSV bytes. Added six synthetic tests for pass/incomplete/invalid output and input-byte identity. This does not authenticate JPL/NAIF provenance or perform astronomical comparisons.

2026-11-13 (planning follow-up): added an opt-in `--plan` mode for explicit two-planet, three-or-more-ET case planning, shared E1 audit rules and seven additional synthetic CLI/validation tests. No astronomical calculation or external data access added.

2026-11-13 (follow-up): corrected planetary-system identity when selecting independent test targets: planet center and system barycenter IDs now map to the same physical planet for coverage counting. Added regression tests without changing astronomical calculations.

2026-11-13: Added original `scripts/audit_spk_e1_matrix.py`, synthetic regressions in `scripts/tests/test_spk_e1_matrix.py`, and safe Python tests in the existing three-platform development CI. Original source stays under the project's [CPAL-1.0 license](../LICENSE), [NOTICE](../NOTICE), and mandatory VirgoStars attribution; no third-party software, datasets, production changes, publication or release.
