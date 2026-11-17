# NAIF DAF/SPK FTPSTR header integrity guard — 2026-11-14

**CPAL-1.0 §3.3 modification record.** The `spk_index.rs` parser now checks a populated NAIF DAF FTP transfer-validation string in the first 1024-byte file record. Original CPAL source notices, LICENSE, NOTICE and **Powered by VirgoStars** attribution remain intact.

## Problem

The existing Rust `inspect_spk(&mut reader)` validates `DAF/SPK ` identification, 2/6 summary dimensions, the binary byte-order label, record addresses and summary chain, but did not check the header's FTP transfer corruption sentinel. NAIF's published DAF Required Reading puts `FTPSTR` at **0-based byte offset 699**, with an exact 28-byte sequence intended to reveal transformations between CR, LF, NUL and other control/high-bit bytes during transfers.

A valid-looking header whose transfer marker had changed could still produce segment metadata. The new guard rejects that specific corruption **before** reading any DAF summary record.

## Behavior

- If the 28-byte field is the exact canonical value, the header is accepted and validated as before.
- If all 28 bytes are NUL (`0x00`) or all are ASCII spaces (`0x20`), treat the optional marker as absent for compatibility with old/hand-built files; this **does not** certify the kernel's transfer integrity.
- Any *other* 28-byte value, including partially filled markers, combinations of NULs and spaces, altered CR/LF/NUL and high-bit bytes, is rejected as `SpkIndexError::InvalidFile("invalid DAF FTP transfer validation string")`.
- The byte string is checked identically for `LTL-IEEE` and `BIG-IEEE` files. No byte-order conversion or dependency is involved.

The safeguard checks only a known header marker, not checksums, kernel provenance, correct scientific coefficients or independently verified ephemeris outputs. A missing marker is **not** interpreted as proof that the file has not been corrupted. Actual kernel licensing is unaffected.

## Deterministic regressions

New module-local tests exercise:

1. Both IEEE byte orders and both single- and double-summary synthetic DAF fixtures.
2. Exact canonical FTPSTR accepted and all-zero/all-space legacy marker accepted.
3. Each of the 28 individual bytes corrupted in turn, including control and extended-byte positions; every case rejected.
4. Truncated `FTPSTR:` and NUL/space mixed fields rejected.

All fixtures are synthetic. **No official NASA/JPL `.bsp` is bundled, loaded or numerically compared to Horizons or the NAIF Toolkit.**

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Run read-only CI against the **exact PR head** (Linux, Windows and macOS). The parser is experimental and does not yet satisfy real-kernel/Horizons independent numerical validation or production release requirements.

## Primary specification

- NAIF, [DAF Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html), *The File Record*, `FTPSTR` 28-byte field at address 699 and sample octal dump.
