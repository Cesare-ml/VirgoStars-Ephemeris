# SHA-256-pinned, offline classic DE441 vs Horizons preflight

**Added 2026-11-14.** Original VirgoStars source under CPAL-1.0
(Exhibit A in the Python files); required attribution **Powered by
VirgoStars**, https://www.virgostars.it/. No external kernel or reference
data is redistributed, downloaded or committed.

## Why this exists

The existing Rust `validate_horizons` example compares **geometric**
ICRF XYZ and velocity against a manually reviewed JPL Horizons CSV.
It checks TDB Julian days, numeric values and required reference
metadata, but it does not itself fingerprint the binary file and CSV
supplied to the comparator. Re-running the same scientific experiment
with accidentally modified input bytes can give misleading evidence.

`scripts/validate_classic_de441_horizons.py` is an offline Python 3
standard-library preflight that requires **two independently
authenticated expected SHA-256 fingerprints** before calling the existing
Rust comparator. The hash is streamed in chunks rather than loading the
multi-GB kernel into memory. This is validation tooling only; **Python
does not implement or duplicate any astronomy**.

The preflight rejects non-files, non-64-hex digests, two identical input
paths, files shorter than **1,000,000,000 bytes** (defensive engineering
heuristic specific to the expected full classic DE441 distribution),
nonclassic DE441 header/magic/version, an altered kernel or reference
hash, nonpositive/nonfinite declared error tolerances, and an output
receipt path which would overwrite either input, an existing receipt,
or a file inside the Git source checkout.

The existing Rust parser remains the authority for complete header,
coverage, body identity, reference CSV metadata, time, frame,
corrections, units, and six-vector finite residual validation.

**Source revision gate (added 2026-11-14):** before running Cargo,
the preflight also requires a separately preapproved **40-hex Git
commit SHA** (`--source-sha`), checks that the script resides at the
actual repository root, checks that `HEAD` matches the approved commit,
and rejects any tracked or untracked changes reported by
`git status --porcelain=v1 --untracked-files=all`.
No uncommitted source code can be silently mixed with the validated
kernel/reference input files. Keep the two external inputs and the
receipt folder **outside** the source checkout. A clean Git tree and
matching commit still require an independent code review and do
not authenticate NASA/JPL material; ignored files and external toolchains
are not covered by this check.

The preflight does not independently verify the authenticity of a CSV
query URL or prove that Horizons actually used the DE441 solution.

## Prepare authentic inputs

1. Obtain a **classic little-endian `de441.eph`**, not a `.bsp` SPK
   or an Astrodienst `.se1`, from a separately approved source.
   Record the original distribution URL, provider, licensing review,
   file size, and **trusted fingerprint from an independent channel**.
   A digest computed only from an unverified local file is insufficient
   to establish authenticity.
2. Independently obtain matched Horizons VECTORS for the **same DE441**
   ephemeris, using `TIME_TYPE=TDB`, `REF_SYSTEM=ICRF`,
   `REF_PLANE=FRAME`, `VEC_CORR=NONE`, `OUT_UNITS=KM-D`;
   document `target` and `center` exactly (system barycenters are
   distinct from planetary centers). Preserve the full original
   Horizons response/API query and the curated `reference.csv`.
3. Save/approve both SHA-256 hashes **before executing** the comparison.
   Record and approve the precise 40-hex Git commit SHA of the Rust
   repository. Ensure `git status --porcelain=v1 --untracked-files=all`
   reports nothing. Agree on two **strictly positive component tolerances**
   in km and km/TDB-day; the script has no defaults and does not approve them.

## Run (Windows 11 PowerShell)

Before comparing, record the reviewed commit:

```powershell
git rev-parse HEAD
git status --porcelain=v1 --untracked-files=all
```

```powershell
py -3 scripts/validate_classic_de441_horizons.py `
  --kernel 'D:\Ephemerides\de441.eph' `
  --reference 'D:\Ephemerides\horizons-reference.csv' `
  --kernel-sha256 '<64-hex trusted kernel fingerprint>' `
  --reference-sha256 '<64-hex reviewed CSV fingerprint>' `
  --position-km '0.01' `
  --velocity-km-per-day '0.0001' `
  --source-sha '<40-hex approved Git commit>' `
  --receipt 'D:\Ephemerides\evidence\de441-horizons-receipt.json'
```

The displayed tolerance values above are **illustrative only**:
choose your approved values before collecting scientific results.
For Linux/macOS, use `python3` and replace the sample paths; argument
names remain identical. A Rust/Cargo installation must be available
for the actual comparison.

The script invokes the **existing** Rust implementation only:

```text
cargo run --locked --quiet -p virgostars-ephemeris-core --example validate_horizons -- <kernel> <reference.csv> <max_position_component_km> <max_velocity_component_km_day>
```

The receipt (schema version **2**) records both input hashes, kernel
basename and bytes, the exact verified Git commit and clean-tree status,
declared component tolerances, command/exit status and full Rust
stdout/stderr. Failures cause nonzero exit; mismatched fingerprints,
unapproved commits and dirty worktrees stop **before Cargo**.

**Preservation:** every receipt is created exclusively (no overwrite).
A destination that already exists, aliases either input, or resolves
inside the Git source checkout is rejected before comparison.
Choose a fresh receipt name for each experiment and keep it outside
the repository, even when a previous experiment failed.

A zero Cargo exit produces the deliberately limited status
`COMPARISON_PASSED_PENDING_INDEPENDENT_PROVENANCE_REVIEW`,
**not** `SCIENTIFIC_VALIDATED`. The JSON flag
`scientific_validated` is always `false`. A human scientific audit
must still confirm source revision, kernel authenticity, matched
independent Horizons solution, exact query/units/reference metadata,
tolerance appropriateness and a multi-epoch/body coverage matrix.
This tool is not proof of Swiss Ephemeris parity, a production
release gate or permission to bundle a third-party kernel.

## Deterministic offline test evidence

The existing read-only Linux Rust development CI already runs
`python3 -m unittest discover -s scripts/tests -p 'test_*.py'`;
it will execute the new tests without a workflow edit. Reproduce:

```sh
python3 -m unittest discover -s scripts/tests -p 'test_classic_de441_horizons_preflight.py'
cargo test --workspace --example validate_horizons --locked
```

The tests fabricate a small **fictional header** only; the test
temporarily relaxes the size preflight *inside the mock unit process*,
never in the production CLI. Their subprocess mocks simulate Git
root, commit and worktree status as well as Cargo, rejecting invalid,
unexpected or dirty source revisions without running astronomical
calculations. Their subprocess mock never calls Cargo
or an authentic JPL file. Test success establishes input integrity and
fail-closed control-flow behavior only; no actual NASA/JPL comparison
has been executed by these tests.

See [the classic DE441 comparator contract](DE441_HORIZONS_OFFLINE_COMPARISON_2026-11-10.md),
[Validation protocol](VALIDATION.md) and [Contributor rules](../AGENTS.md).
