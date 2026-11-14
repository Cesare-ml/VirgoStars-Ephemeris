# Offline SPK geometric state validation against independent NAIF/SPICE

**Created 2026-11-13. Status: comparator implemented; no real-kernel SPK scientific comparison has been run or claimed.**

This procedure is an experimental, deterministic, **offline** reference comparison using `SpkKernelSet<File>` and externally supplied NAIF/SPICE `spkezr` results. The underlying SPK modules are integrated on `dev/ephemeris` through [#148](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/148), [#150](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/150), [#157](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/157), and [#159](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/159); the Rust comparison harness is integrated through [#160](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/160). The comparator does not import external SPICE code or kernels, make network requests, or claim independent astronomical results from its synthetic tests.

## Reference contract

Use the NAIF Toolkit's independent SPICE implementation `spkezr` with **exactly the same set of SPK files in the same load order** as the Rust reader. Specify all of:

- **Target and observer:** exact numeric NAIF IDs (for example `301` Moon, `399` Earth), not barycentric alternatives with different physical meaning.
- **Epoch:** `et` in **TDB seconds past J2000**, not JD, TT, UTC or local time. Do not let an external civil-time conversion silently change the comparison.
- **Frame:** `J2000` (fixed NAIF frame 1). All required SPK segments must be compatible with the Rust reader's existing J2000 Type-2/3 limitation. Do not equate unrelated frames.
- **Aberration correction:** `NONE` (**geometric** target minus observer), not `LT`, `CN`, `LT+S` or apparent state.
- **Units:** **km** for X/Y/Z and **km/s** for VX/VY/VZ, as returned by NAIF `spkezr`.
- **Provenance:** NAIF Toolkit version, independent raw output/query or script, generated UTC, ordered SPK filename, byte length and SHA-256 digest. Record the actual origin/rights of each kernel and the reference output. Do not commit SPK files or generated external reference output without separate rights review.

Source: [official NAIF `spkezr_c` reference](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/cspice/spkezr_c.html), which specifies `et` in TDB seconds past J2000, `NONE` as geometric, and return units in km and km/sec.

## Required local CSV

Create a user-reviewed local UTF-8 file. The example below shows **placeholders, NOT real reference vectors**. Each `<64_HEX_SHA256>` must be replaced by the actual local file digest, and row values must be independently obtained:

```text
# provider=NAIF_SPICE
# method=spkezr
# time_scale=TDB
# frame=J2000
# corrections=NONE
# units=KM-S
# toolkit_version=<actual NAIF toolkit version>
# generated_utc=<actual UTC timestamp>
# reference_source=<retained path or URL to the original NAIF output/script>
# ordered_kernel_sha256=<64_HEX_SHA256_OF_FIRST_FILE>;<64_HEX_SHA256_OF_SECOND_FILE>
et_tdb_seconds,target_id,center_id,x_km,y_km,z_km,vx_km_s,vy_km_s,vz_km_s
<ET_TDB_SECONDS>,301,399,<X>,<Y>,<Z>,<VX>,<VY>,<VZ>
```

For a single kernel, use just one digest with no semicolon. Each row contains exactly nine finite fields; target/center IDs must be integers and differ, and a target/center/ET tuple may occur only once. Arbitrary ordering of different epochs and target pairs is permitted, but duplicate tuples are rejected. The example refuses unknown frame/time scale/correction/unit declarations, invalid/missing metadata, malformed hashes, duplicate metadata, nonfinite vectors and nonfinite/zero/negative acceptance thresholds. **The hashes are required declarations, not automatically verified by the Rust example.** Run the separate standard-library Python SHA-256 preflight below to compare every declared digest to the actual on-disk SPK bytes in load order. A human must still authenticate the original independent SPICE output and file provenance before treating a comparison as scientific evidence. The parser cannot authenticate the declared provider or catch fabricated reference numbers.

A concise optional external check using an already installed **independent NAIF/SPICE toolkit** (illustrative, not included as a repository dependency) is:

```python
import spiceypy as spice
ordered_kernels = ["/path/to/first.bsp", "/path/to/second.bsp"]
for path in ordered_kernels:
    spice.furnsh(path)
state, _light_time = spice.spkezr("301", 0.0, "J2000", "NONE", "399")
print(state)  # X/Y/Z km and VX/VY/VZ km/s
```

This illustration **does not contain** independent results; `spiceypy` is optional outside the Rust project and is not used by CI. Retain a reproducible script with explicit kernel versions. Compare only results generated from matching files and load order.

To record local file hashes without adding libraries or bundled data: Windows PowerShell `Get-FileHash -Algorithm SHA256 path-to-kernel.bsp`; Linux `sha256sum path-to-kernel.bsp`; macOS `shasum -a 256 path-to-kernel.bsp`. Record *each path and digest in load order*. Then run the reproducible byte-level preflight included with this repository:

```bash
# Python 3.10+, standard library only; no internet, SPICE dependency or SPK file download.
python scripts/verify_spk_kernel_hashes.py \
  /path/to/reviewed/spk-reference.csv \
  /path/to/first.bsp /path/to/second.bsp
python -m unittest discover -s scripts -p 'test_*.py'
```

The preflight streams each kernel in bounded chunks, hashes its **actual bytes** using SHA-256, and compares them with the corresponding CSV digest **in the supplied load order**. A missing file, changed file, swapped order, invalid hash or wrong kernel count produces a nonzero exit. Python 3.10 or newer is required for the type annotations. Run this gate against the exact paths passed to the Rust comparator, without replacing the files between checks. The standalone Rust comparator still checks only SHA-256 declaration syntax/count: **it does not invoke this Python preflight automatically**. Matching hashes cannot prove that the CSV vectors were generated by NAIF/SPICE or that the acceptance limits are scientifically justified.

## Running the comparator

After the SHA-256 preflight above succeeds, keep the same kernel files and exact order for the numerical comparison:

```bash
cargo test --workspace --example validate_spk_reference --locked
cargo run --example validate_spk_reference --   /path/to/reviewed/spk-reference.csv   <approved-max-position-component-error-km>   <approved-max-velocity-component-error-km-per-second>   /path/to/first.bsp   /path/to/second.bsp
```

The example opens the files **in exactly the listed order**, validates their DAF indexes, and calls `SpkKernelSet::relative_state` for each row. It reports all six **signed residuals** (`Rust − NAIF`), maximum absolute component errors, three-dimensional residual norms, largest observed errors and the number of failures. Its exit status is nonzero if the reference contract is invalid, the kernel load fails, a required segment is unsupported/missing, a result is nonfinite, or **any** component error exceeds the user-supplied positive limits. Numeric tolerances must be explicitly approved for the chosen scientific test; there are no implicit accuracy targets and reported component maxima are the acceptance gates, with vector norms provided as diagnostics.

**A passing CLI run still requires provenance review:** same SPK binaries and load order, verified actual digests (the code does not compute hashes), authentic SPICE reference source and explicit acceptance tolerances. A CI pass without external kernels cannot be labeled astronomical accuracy validation.

## Test evidence and limits

Nine deterministic tests in the Rust example verify the reference CSV parser and signed residual arithmetic, duplicate controls and numerical failures. Seven additional Python `unittest` regressions cover two-file success, swapped load order, altered bytes, missing files, count mismatch, malformed/duplicate declarations and uppercase digests. They use **artificial numbers**, not NAIF outputs. The existing read-only CI now runs `cargo test --workspace --example validate_spk_reference --locked` on **Ubuntu, Windows and macOS** in addition to the existing DE441 reference comparator. The example is never run with external kernels on CI and never fetches from the internet. This feature adds no Rust dependencies.

The next true validation milestone is a **manually executed and recorded actual NAIF/SPICE comparison** using a matched real JPL kernel, source sha256s, reference script/toolkit version, a multi-body/epoch test matrix, approved limits and a persistent evidence receipt. Mark the milestone `IMPLEMENTED_UNVERIFIED` until such a receipt exists.

## CPAL-1.0 modification record

2026-11-13 (follow-up): Added `scripts/verify_spk_kernel_hashes.py` and seven synthetic standard-library unit tests, documented the separate cryptographic preflight and enabled it in read-only CI. No external kernel files or new Rust/Python runtime dependencies.

2026-11-13: Added original `crates/ephemeris-core/examples/validate_spk_reference.rs` reference harness, extended `.github/workflows/rust-dev.yml` with read-only three-platform parser/regression checks, and documented the scientific gates. Original VirgoStars work remains under [CPAL-1.0](../LICENSE), with the required [NOTICE](../NOTICE) and [attribution and credits](LICENSING_AND_CREDITS.md). No third-party kernel data, external source code, deployments, releases or production changes.

## Reproducible independent reference generation and digest verification (2026-11-13)

The add-on script [`scripts/spk_naif_reference.py`](../scripts/spk_naif_reference.py) builds on the existing SHA-256 preflight with independently generated SPICE references and an offline one-command comparison. It is a scientific **test utility**, not a second astronomical implementation. It calls an independently installed **NAIF Toolkit through SpiceyPy** *only when generating reference vectors*. Rust remains the sole source for all VirgoStars production astronomy.

Generate a reference from already obtained external kernels, using explicit **TDB seconds past J2000**. File order matters. The utility never downloads kernels, converts UTC, invents epochs, or bundles external data.

```powershell
# Run in the repository root. Install spiceypy in your own isolated
# Python environment before using generate; CI mocks the SPICE calls.
python scripts/spk_naif_reference.py generate `
  --kernel "C:\\kernels\\de440s.bsp" `
  --case "301:399:0" `
  --case "301:399:86400" `
  --case "399:0:0" `
  --output "C:\\my-validation\\spk-reference.csv"

# Independent provenance preflight using Python stdlib only.
python scripts/spk_naif_reference.py verify `
  --reference "C:\\my-validation\\spk-reference.csv" `
  --kernel "C:\\kernels\\de440s.bsp"

# One-command, offline verify + Rust comparison + verify again.
# Choose scientifically justified positive limits; no accuracy defaults.
python scripts/spk_naif_reference.py compare `
  --reference "C:\\my-validation\\spk-reference.csv" `
  --kernel "C:\\kernels\\de440s.bsp" `
  --position-limit-km <approved_position_component_km> `
  --velocity-limit-km-s <approved_velocity_component_km_per_second>
```

For bash, replace PowerShell continuation characters with backslashes. Add `--kernel` arguments in the same order to all commands for a multi-kernel comparison. The independent `verify` subcommand remains available when only hash checking is desired. `--case` can be repeated for multiple finite epochs and distinct target/center pairs; duplicate triples and same-body tautologies are rejected. The example's target and observer IDs are merely candidate cases; they succeed **only if the chosen kernels actually cover those bodies/epochs using supported frame-1 SPK types 2 or 3**.

The new `compare` subcommand requires **explicit, finite, strictly positive** position/velocity component tolerances; verifies the ordered kernel names, sizes, hashes and generator script fingerprint **before** running `cargo run --offline --locked --example validate_spk_reference`; then rechecks both original CSV bytes and ordered kernel digests even if Rust fails. It passes values as individual subprocess arguments, never through a shell, and propagates Rust's nonzero exit. Cargo's `--offline` prevents dependency downloads during this check. With the added source-code fingerprint, users of an older generator revision must regenerate their reference CSV under the new version before the `compare` command can accept it. The separate standalone Rust comparator remains usable but **does not itself perform these pre/post SHA-256 checks**. This reduces accidental use of stale/corrupted inputs but **does not provide an atomic filesystem snapshot**: externally modified files after the final check, or an attacker controlling both references and hashes, remain outside this tool's guarantee.

The generator uses `spkezr(target, et, "J2000", "NONE", observer)`, records its NAIF toolkit version, the UTC creation timestamp, full-precision floating-point output, and an ordered manifest of kernel **names, byte sizes and SHA-256 digests**. It also records the **SHA-256 of the exact generator script source**. Its `verify` command rehashes every file and the current generator script and rejects missing, substituted, reordered, truncated or same-length-corrupted kernel bytes. On `generate`, it checks source hashes before and after independent SPICE evaluation and clears the global SPICE kernel pool even on errors. The output CSV is written only once all requested vectors and hash checks succeed; an existing output is not overwritten without explicit `--overwrite`.

This is a stronger reproducibility gate, but **hash matching is not scientific validation by itself**: an attacker can regenerate all hashes and vector values. Scientific acceptance still requires an authentic and reviewed NAIF/SPICE source and matching loaded kernels, an independently approved test-case matrix and numerical error thresholds, and a recorded Rust comparator run. A real-kernel comparison has **not** yet been executed in CI.

Offline mocked Python regressions run in Rust development CI on Ubuntu, Windows and macOS:

```bash
python3 -m unittest discover -s scripts/tests -p 'test_spk_naif_reference.py'
```

These tests exercise manifest integrity, load order, duplicate/invalid time/IDs, nonfinite SPICE output, cleanup on failures and corrupt bytes; seven additional mocked cases cover the single-command offline Rust invocation, invalid thresholds, pre/post file tampering and nonzero process exits. They **do not** call NAIF Toolkit or validate JPL numerical parity.

### Modification record

2026-11-13 (follow-up): added an offline `compare` orchestration command to the original `spk_naif_reference.py`, seven mocked safety tests, documentation for fail-closed verification before/after Rust and source-fingerprint compatibility. No data, deployment or release added.

2026-11-13: added original validation-only `scripts/spk_naif_reference.py`, `scripts/tests/test_spk_naif_reference.py`, and read-only CI checks on three platforms. The source preserves the project CPAL-1.0/Exhibit A and VirgoStars attribution. No third-party source, kernel binary, network call, production component or runtime dependency was added to the Rust engine.

## One-command, fail-closed local scientific-validation receipt (2026-11-13)

The same [`scripts/spk_naif_reference.py`](../scripts/spk_naif_reference.py) now has a `run` mode to connect all three independent stages without copying them into a second validation tool:

1. Call independently installed NAIF/SPICE through SpiceyPy to generate a fresh **J2000, geometric, TDB seconds past J2000, km and km/s** reference CSV from locally obtained SPK files. The script records the toolkit version, exact kernel names/sizes/hashes, generator source hash and UTC timestamp.
2. Recompute and check source and kernel hashes, then invoke the canonical **Rust** comparator with `cargo run --offline --locked --example validate_spk_reference -- ...`. The comparison is offline and uses **caller-approved finite positive** per-axis position/velocity tolerances; no default accuracy thresholds are assumed.
3. **Always recheck** the generated CSV bytes and every source/kernel hash after the Rust subprocess returns, fails, or cannot start. Require **Rust exit code 0** as an additional condition before writing a **new, non-overwriting JSON receipt** containing the reference digest, kernel manifests and order, exact Rust command, all comparator stdout/stderr, toolchain version reported by SPICE, case count, user-supplied tolerances and audit status `COMPARATOR_PASS_PROVENANCE_REVIEW_PENDING`.

### Operator command (Windows PowerShell)

```powershell
# Prerequisites: Rust toolchain, Python 3.10+, independently installed
# SpiceyPy and a legally obtained JPL SPK file. Nothing is downloaded.
# Do not put a receipt or external SPK kernel inside the Git repository.
python scripts/spk_naif_reference.py run `
  --kernel "C:\\kernels\\de440s.bsp" `
  --case "301:399:0" `
  --case "301:399:86400" `
  --case "399:0:0" `
  --reference "C:\\my-validation\\spk-2026-11-13.csv" `
  --receipt "C:\\my-validation\\spk-2026-11-13.receipt.json" `
  --max-position-km "<approved-positive-km>" `
  --max-velocity-km-s "<approved-positive-km-per-second>"
```

Replace angle-bracket values with **review-approved numbers**, without quotes if preferred. Adapt the paths, kernels and test matrix to the actual binary's segment coverage. Additional `--kernel` flags preserve load order; additional `--case` flags add comparison rows. This command is **illustrative**: no test on a real kernel has been executed in this contribution.

The tool **never overwrites** an existing reference or receipt, or writes a *success receipt* after a failed Rust comparator or after either file changes. On a comparison failure the generated CSV may remain on disk for troubleshooting; no success receipt is produced. The receipt is an **audit candidate, not an accuracy certificate**: `scientific_accuracy_certified=false` and `provenance_review_required=true` remain explicit even on successful comparisons. Operator review must still establish the authenticity of the kernel's source, real NAIF reference implementation, independent tolerance policy, JPL solution version, and quantitative error budget. Human review is required before using results in production.

This mode is **not an astronomical implementation in Python**: SpiceyPy runs only to generate an independent reference, and all VirgoStars calculations and residual acceptance remain in Rust. No real JPL data or SpiceyPy dependency is added to CI.

### Tests and security boundaries

The existing `scripts/tests/test_spk_naif_reference.py` suite now includes five additional deterministic **mock-SPICE and mock-Cargo** integration regressions. They prove that an unsigned JSON evidence receipt is created only after an exit-zero comparator plus rehashed inputs, and that failed comparison, mutated kernel bytes or changed CSV bytes prevent any success receipt. Four added failure-path regressions verify that changed kernel/CSV bytes are still detected when Rust reports failure or cannot start, while an unmodified missing executable retains its original error. These are **synthetic workflow tests**: they do not establish SPK real-kernel accuracy. The JSON receipt is not cryptographically signed; every file/source hash is a digest rather than an identity or authenticity signature; provenance still requires independent review.

**Modification record:** 2026-11-13 — updated the original CPAL-1.0 Python validation helper and tests; linked this orchestration workflow without adding a new duplicate script, extra runtime dependency to Rust, dataset, secret, release, deployment or automatic production action.

**Modification record (2026-11-14):** rehash reference CSV, generator source and ordered local kernels in a `finally` block **even when Cargo returns nonzero or raises an operating-system error**. No comparator failure or input mutation can produce a success receipt. This does not make the JSON cryptographically signed, make the files immutable, authenticate a NAIF toolkit/kernel or demonstrate real-data positional accuracy.
