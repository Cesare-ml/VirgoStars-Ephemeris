# Experimental Rust DAF/SPK metadata index

**2026-11-12 — implementation note.** This module is a first step toward Rust SPK support, **not** a working SPK state evaluator and **not** independent NASA/JPL numerical validation.

## What works

`spk_index::inspect_spk(&mut reader)` reads an **externally supplied**, seekable `DAF/SPK ` binary stream without loading its ephemeris coefficient arrays. It inspects:

- Exactly 1024-byte DAF physical records with `ND=2`, `NI=6`;
- file records encoded as `LTL-IEEE` or `BIG-IEEE` (including the integer pointers in the file record);
- a forward-linked list of DAF summary records, each holding at most 25 SPK descriptors;
- TDB coverage in **seconds past J2000**, NAIF target/center/frame/type IDs, and the initial/final **1-based 8-byte-word addresses** of each segment.

It checks the file ID, dimensions, byte order, whole-record alignment, free-address range, segment coverage and data-span bounds, summary record count, link consistency and cycles. The reader preserves the **original forward order** of descriptors, which is necessary for later NAIF segment-priority semantics. A segment with an unsupported frame or type is **indexed as metadata**, not silently dropped or used for calculations. This allows a future evaluator to fail closed if the *selected* segment is unsupported.

The `SpkIndex` intentionally does not return any positions or velocities. It does not parse name/comment records as human-readable metadata, verify full SPK type-2/3 record layout, compose NAIF center chains, interpret non-J2000 frames, resolve overlapping multi-kernel priorities, or convert UTC/TT to TDB. Populated DAF FTPSTR transfer-sentinel fields are verified byte-for-byte; legacy all-NUL or all-space sentinel fields remain accepted. SPK type IDs are recorded but **not executed**.

## Usage

```rust
use std::fs::File;
use virgostars_ephemeris_core::spk_index::inspect_spk;

# fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut kernel = File::open("/path/to/legally-obtained/de440s.bsp")?;
let index = inspect_spk(&mut kernel)?;
for segment in &index.segments {
    println!(
        "target {} center {} frame {} type {} ET {}..{}",
        segment.target_id, segment.center_id,
        segment.frame_id, segment.spk_type,
        segment.start_tdb_seconds, segment.end_tdb_seconds
    );
}
# Ok(())
# }
```

No kernel downloads, filesystem searches or production integrations occur inside the module. The caller supplies the source.

## Source specification and engineering evidence

The structure and offsets follow NAIF's published format descriptions:

- [DAF Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html) — 1024-byte file records, file header offsets, summary/name record structure and descriptor link fields.
- [SPK Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html) — DAF/SPK convention and segment identifiers.

These documents are format references only: no NAIF Toolkit code, Swiss Ephemeris code, NASA kernel data, or third-party source has been copied. The public SPK file format is **not** a grant to redistribute kernels.

Six original synthetic tests cover little- and big-endian descriptors (including a two-record chain), malformed file header and truncation, invalid/cyclic pointers, nonintegral summary controls, invalid epochs/address/type, and explicitly indexed unsupported types/frames. These tests establish parsing behavior, **not** scientific correctness against a real JPL SPK file.

Run the existing read-only CI: `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo test --workspace --release --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, Rustdoc, Windows GNU compilation and native Windows/macOS checks. Independent NAIF/Horizons numerical parity still requires a legitimate external kernel with reproducible hashes and published comparison settings.

## Next engineering milestones

1. Read and validate type-2/type-3 coefficient record trailers and epoch coverage from the indexed address spans.
2. Use the already-existing Rust `chebyshev` evaluator to compute SPK states and velocities (**km**, **km/s**) at explicit TDB seconds past J2000, rejecting unsupported frames/types.
3. Implement deterministic file-order and kernel-load-order priority and cycle-safe target-to-center resolution; test segment overlap/boundaries.
4. Independently compare named real-kernel states to identical NAIF/Horizons queries at reviewed, explicit tolerances before integration into `astrological-functions`.

## CPAL-1.0 modification record

2026-11-12: added `crates/ephemeris-core/src/spk_index.rs`, exported it from `lib.rs`, and documented this initial metadata-only reader. Original source remains CPAL-1.0 with the required VirgoStars attribution; [LICENSE](../LICENSE), [NOTICE](../NOTICE), [credits](LICENSING_AND_CREDITS.md). All SPK kernel files remain separately obtained and licensed.

## 2026-11-14 — reject coefficient/metadata record overlap

The DAF summary list may interleave later summary/name pairs with earlier
coefficient records as an SPK grows. The original indexer verified that
segment addresses fit inside the physical file and precede the declared free
address, but that condition alone does **not** prove that an address points
to coefficient data. A malformed descriptor could reference a summary record
or its immediately following name record.

After walking the complete forward summary chain, `inspect_spk` now treats
**both records of every visited summary/name pair** as reserved metadata.
It rejects:

- Any segment whose inclusive 1-based DAF word-address interval touches
  one of those physical metadata records, including a span that starts in a
  name record and continues into ordinary data.
- A corrupt summary link that reuses another summary's name record as a
  summary, causing the metadata pairs to overlap.

The check uses sorted physical record numbers and a binary search for the
first potentially intersecting metadata record per segment. Its cost is
bounded by the number of summary records and descriptors, and it does not
load coefficient arrays. Valid data records on either side of later
summary/name pairs remain indexable; the index still preserves segment order.

Regression tests cover summary, name, crossing-boundary and linked-summary
alias attempts in **both LTL-IEEE and BIG-IEEE** synthetic files, plus
valid-file controls. The error remains `SpkIndexError::InvalidFile`;
there are no public API, data-format, crate-dependency or runtime network
changes.

This is a **structural safety check**, not independent validation of real
JPL/NAIF kernel contents, record type-2/type-3 interpolation, reference
frames, center chains or astronomical accuracy. At the time of this earlier change, it did not detect arbitrary
coefficient-segment overlaps, checksum corruption or separately reserved
comment records. A subsequent address-span guard is documented below.

Reproduce from the Rust workspace root:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked spk_index::tests
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

Modification date: 2026-11-14. The covered Rust code retains its CPAL-1.0
Exhibit A notice and VirgoStars attribution (see [LICENSE](../LICENSE),
[NOTICE](../NOTICE), and
[LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md)). No copied NAIF code
or third-party binary kernel data was introduced. Confirm all CI jobs on the
exact feature-branch commit before integration.


## 2026-11-14 — reject aliasing between different DAF data arrays

The NAIF [DAF Required Reading](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html) describes appending data arrays at the next free word address. Each SPK descriptor identifies an **inclusive** range of 1-based, eight-byte DAF words. Previous index checks excluded summary/name metadata but could still accept two otherwise valid descriptors that referenced the **same coefficient words**, for example `513..516` and `515..518`.

`inspect_spk` now additionally sorts all segment **word spans** by initial address and rejects any neighboring spans where the next start is less than or equal to the previous inclusive end. This detects identical, contained and partial overlaps across summary records and targets. It does **not** reject separate arrays merely because they share one 1024-byte **physical record** (for example `513..516` and `517..520`). It preserves the original segment descriptor order and SPK precedence in the returned index; sorting is used for validation only.

New deterministic tests cover four overlapping address arrangements across distinct summary records in both LTL-IEEE and BIG-IEEE, unchanged valid kernels, and adjacent arrays sharing the same data record without intersecting in words. These fixtures are wholly synthetic and metadata-only. The failure is `SpkIndexError::InvalidFile("overlapping DAF segment data arrays")`; no kernel contents are read and no additional dependencies are introduced.

Verification: `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and `cargo clippy --workspace --all-targets --locked -- -D warnings` in read-only CI, including native macOS and Windows where available. Independent NASA/JPL state-vector comparison and legal review for externally obtained kernels remain separate requirements.

Modification date: **2026-11-14**. The original CPAL-1.0 license, Exhibit A, required VirgoStars attribution and third-party exclusions remain unchanged. This change adds no external source code or kernel data.
