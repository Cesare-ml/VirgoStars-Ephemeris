# DAF/SPK: protect file header and reserved comment records

**Modification:** 2026-11-14. Original VirgoStars Rust code remains under [CPAL-1.0](../LICENSE), with applicable notices in [NOTICE](../NOTICE) and [LICENSING_AND_CREDITS.md](LICENSING_AND_CREDITS.md).

## Problem

A NAIF Double Precision Array File (DAF) contains a file header (physical record **1**), then **zero or more reserved/comment records** (physical records **2 through FWARD−1**), then the first summary/name pair and its element records. Segment descriptors contain **one-based DAF word addresses**, not physical record indexes.

The experimental Rust `spk_index::inspect_spk` already rejected segment spans overlapping a *summary* or *name* record, but accepted a descriptor whose initial and final word addresses referenced a reserved comment record. Such content is not array coefficient data and must not be exposed as a valid segment.

The change converts a segment's initial DAF word address to its physical record number, then rejects it if **`first_record < FWARD`**, where `FWARD` is the first summary record number from the file header. This protects the entire initial non-element region; the existing later check continues protecting every summary/name pair, including those interspersed with data later in the file.

This is a narrow fail-closed metadata validation. No altered ephemeris evaluation, date/time models, record precedence, numerical interpolation, public API, or additional dependency. Unsupported SPK types and reference frames remain explicitly unsupported by the actual state evaluator.

## Deterministic tests

The new Rust tests construct **entirely synthetic** DAF/SPK binaries and verify both `LTL-IEEE` and `BIG-IEEE`:

1. A valid file with one comment record at physical record **2** and summary/name/data at **3/4/5** continues to index normally. Replacing the descriptor span with **DAF words 129–132 or 249–252**, both inside the reserved comment record, is now rejected with an explicit `SpkIndexError::InvalidFile`.
2. A valid file with **no** comment records (summary/name/data at **2/3/4**) is still accepted, preventing an accidental blanket ban on early array records.
3. A valid file with **four** comment records (summary/name/data at **6/7/8**) is still accepted; its descriptor is rejected only when it falsely points into its final comment record **5**.

Segment summaries, free-word pointers, array addresses and physical record relocations are adjusted in each fixture, so tests exercise full public `inspect_spk` parsing rather than passing a fabricated index.

## Format reference and scientific limits

[NAIF DAF Required Reading — DAF Organization, Reserved Records and Element Records](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html) states that reserved records are the comment area, while array elements reside in element records between summary/name pairs or after them. All numeric test data is synthetic; no NAIF/SPICE source code or actual JPL kernel is bundled.

These tests verify **file structure integrity**, not astronomical accuracy. They do not establish NASA/JPL/Horizons agreement or production-ready SPK compatibility.

## Reproduction

```sh
cargo fmt --all -- --check
cargo test --workspace spk_index::tests::rejects_segments_aliasing_reserved_comments_in_both_endiannesses
cargo test --workspace spk_index::tests::validates_variable_comment_area_lengths_without_rejecting_valid_data
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Check the GitHub Actions run for the **exact PR head** on Linux (Rustfmt, tests, strict Clippy), macOS and Windows (native Rust tests). External kernels are not needed.
