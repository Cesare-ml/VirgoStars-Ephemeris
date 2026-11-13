# Offline SPK geometric state validation against independent NAIF/SPICE

**Created 2026-11-13. Status: comparator implemented; no real-kernel SPK scientific comparison has been run or claimed.**

This procedure is an experimental, deterministic, **offline** reference comparison using `SpkKernelSet<File>` and externally supplied NAIF/SPICE `spkezr` results. It is stacked on draft PRs [#148](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/148), [#150](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/150), [#157](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/157), and [#159](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/159). Review dependencies in order. The comparator does not import external SPICE code or kernels, make network requests, or claim independent astronomical results from its synthetic tests.

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

For a single kernel, use just one digest with no semicolon. Each row contains exactly nine finite fields; target/center IDs must be integers and differ, and a target/center/ET tuple may occur only once. Arbitrary ordering of different epochs and target pairs is permitted, but duplicate tuples are rejected. The example refuses unknown frame/time scale/correction/unit declarations, invalid/missing metadata, malformed hashes, duplicate metadata, nonfinite vectors and nonfinite/zero/negative acceptance thresholds. **The hashes are required declarations, not automatically verified** by the dependency-free Rust example. An independent human must compare actual on-disk hashes and original SPICE outputs before treating a comparison as scientific evidence. The parser cannot authenticate the declared provider or catch fabricated reference numbers.

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

To record local file hashes without adding libraries or bundled data: Windows PowerShell `Get-FileHash -Algorithm SHA256 path-to-kernel.bsp`; Linux `sha256sum path-to-kernel.bsp`; macOS `shasum -a 256 path-to-kernel.bsp`. Record *each path and digest in load order*. You must manually check the manifest matches the CSV metadata.

## Running the comparator

```bash
cargo test --workspace --example validate_spk_reference --locked
cargo run --example validate_spk_reference --   /path/to/reviewed/spk-reference.csv   <approved-max-position-component-error-km>   <approved-max-velocity-component-error-km-per-second>   /path/to/first.bsp   /path/to/second.bsp
```

The example opens the files **in exactly the listed order**, validates their DAF indexes, and calls `SpkKernelSet::relative_state` for each row. It reports all six **signed residuals** (`Rust − NAIF`), maximum absolute component errors, three-dimensional residual norms, largest observed errors and the number of failures. Its exit status is nonzero if the reference contract is invalid, the kernel load fails, a required segment is unsupported/missing, a result is nonfinite, or **any** component error exceeds the user-supplied positive limits. Numeric tolerances must be explicitly approved for the chosen scientific test; there are no implicit accuracy targets and reported component maxima are the acceptance gates, with vector norms provided as diagnostics.

**A passing CLI run still requires provenance review:** same SPK binaries and load order, verified actual digests (the code does not compute hashes), authentic SPICE reference source and explicit acceptance tolerances. A CI pass without external kernels cannot be labeled astronomical accuracy validation.

## Test evidence and limits

Nine deterministic tests in the new Rust example verify the reference CSV parser and signed residual arithmetic, duplicate controls and numerical failures. They use **artificial numbers**, not NAIF outputs. The existing read-only CI now runs `cargo test --workspace --example validate_spk_reference --locked` on **Ubuntu, Windows and macOS** in addition to the existing DE441 reference comparator. The example is never run with external kernels on CI and never fetches from the internet. This feature adds no Rust dependencies.

The next true validation milestone is a **manually executed and recorded actual NAIF/SPICE comparison** using a matched real JPL kernel, source sha256s, reference script/toolkit version, a multi-body/epoch test matrix, approved limits and a persistent evidence receipt. Mark the milestone `IMPLEMENTED_UNVERIFIED` until such a receipt exists.

## CPAL-1.0 modification record

2026-11-13: Added original `crates/ephemeris-core/examples/validate_spk_reference.rs` reference harness, extended `.github/workflows/rust-dev.yml` with read-only three-platform parser/regression checks, and documented the scientific gates. Original VirgoStars work remains under [CPAL-1.0](../LICENSE), with the required [NOTICE](../NOTICE) and [attribution and credits](LICENSING_AND_CREDITS.md). No third-party kernel data, external source code, deployments, releases or production changes.

## Reproducible independent reference generation and digest verification (2026-11-13)

The add-on script [`scripts/spk_naif_reference.py`](../scripts/spk_naif_reference.py) closes the previous **manual-only** manifest step. It is a scientific **test utility**, not a second astronomical implementation. It calls an independently installed **NAIF Toolkit through SpiceyPy** *only when generating reference vectors*. Rust remains the sole source for all VirgoStars production astronomy.

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
