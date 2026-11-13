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
