# Classic DE441 safety and boundary integration — 2026-11-14

**Status:** integration-only, draft verification branch; **do not merge this duplicate PR**.
**Base:** `dev/ephemeris` at `1467998aa63f2d5040607e5876db501543b025ec`.
**License:** existing Rust source CPAL-1.0 with original Exhibit A preserved.
**Modification date:** 2026-11-14.

## Rationale

The four independently green, unmerged proposals touch the same classic
binary DE441 reader or directly test its epoch arithmetic. Each individual
PR passing CI does **not** establish that their modifications work together.
This branch combines the exact scoped changes of the three runtime fixes
against one contemporary `dev/ephemeris` base and adds the fourth PR's
test-only regressions. The goal is to run cross-feature CI before deciding
on a human-reviewed merge order; it is *not* a substitute for the original
reviewable PRs and should not itself be integrated into production.

| Original PR | Pinned source/test head | What is combined |
|---|---|---|
| [#174](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/174) | `059d92ad0e75cfb2a5f50d7d08aef075a2a302b9` | 16 MiB max record-cache size before allocation and two tests |
| [#182](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/182) | `417dbd3a0526bd7a093e22d5f3ce495ed9eb2d63` | Four synthetic negative, zero and distant TDB JD regressions |
| [#187](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/187) | `9f5ca08d3398b6e3ae32a479efac18c34f2b2bb5` | Representable granule-grid guard, boundary binary search, seven tests |
| [#193](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/193) | `4f3089aafa7acfbb944941ffec1c1fbd9f482141` | Record boundary binary search using original TDB JD and three tests |

The 16 test functions across four copied source test files are **synthetic
regression oracles**; any existing workspace tests add further coverage.
No new runtime APIs, external kernel files, dependencies or build actions.

## Combined invariant

For a supported classic little-endian DE441 kernel:
1. Reject a record-size declaration above the defensive 16 MiB limit
   *before* allocating the reusable coefficient cache.
2. Require distinguishable binary64 Julian-date boundaries for all
   nonempty granule series at the first and final principal record.
3. Select the principal record from the original input Julian Date TDB via
   monotonically ordered header-derived boundaries. An exact interior
   boundary belongs to the subsequent record and the last coverage epoch
   remains in the final record.
4. Select a series granule using its binary64 boundaries (not division
   rounded to the wrong index). Exact interior boundaries belong to the
   subsequent granule, the final endpoint belongs to the last.
5. Return geometric ICRF Cartesian positions in km, velocities in km/day,
   with explicit target relative to center. No scientific model, timescale
   conversion, light time or apparent-correction changes.

### Expected performance / risk

The record and granule index selection change from approximate constant
work to bounded binary searches, respectively O(log record count) and
O(log granules). This favors correct boundary semantics; no benchmarks or
performance gains are claimed. Whole-file coefficient integrity and
independent NAIF/JPL/Horizons numerical parity remain unverified.
Multiple draft PRs edit the same production path, so avoid an automatic
merge queue or accepting this duplicate branch into `dev/ephemeris`.

## Read-only verification

The existing authorized PR workflow should run Linux Rust formatting,
workspace/debug/release tests, strict Clippy, rustdoc, and native Windows
and macOS checks at the **exact integration branch head**. Relevant manual
commands:

```sh
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --release
cargo clippy --workspace --all-targets --locked -- -D warnings
```

No external DE441 kernel is bundled or downloaded. A successful CI run is
evidence that these source/test changes coexist and pass the automated
synthetic checks, not of physical accuracy or readiness for production.

## Integration handoff

Review original PRs individually and merge them into `dev/ephemeris` only
with the owner's approval. Rebase/recheck their exact current heads and
expected conflicts after every merge, especially overlapping hunks in
`de441.rs`. This verification branch is ephemeral evidence and should
remain unmerged. Do not alter `main`, publish packages, deploy consumers or
change licensing as part of this check.
