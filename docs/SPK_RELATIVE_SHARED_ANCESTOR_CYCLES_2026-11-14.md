# SPK relative states: stop at the nearest required shared ancestor

**Modification:** 2026-11-14. Original VirgoStars Rust source and these modifications retain the [CPAL-1.0 license](../LICENSE), [NOTICE](../NOTICE) and [required attribution](LICENSING_AND_CREDITS.md).

## Failure in the previous resolution path

`spk_relative_state::resolve_relative` computes geometric target-minus-center position and velocity in **NAIF frame 1 (J2000/ICRF)**, in **km and km/TDB second** at ET seconds since J2000. The one-kernel and ordered multi-kernel APIs both reuse this resolver.

The solver already stops **numerical accumulation** at the *nearest shared ancestor*. But the metadata discovery step previously traversed the **complete ancestry of both requested bodies** before checking whether they had already met. If an upstream metadata cycle existed **above** their nearest shared ancestor, it returned `SpkRelativeError::Cycle` even though none of the cycle's links were needed for the requested state.

Example with synthetic body IDs:

```
301 ─┐
     ├──> 399 ──> 3 ──┐
302 ─┘       ^        │
              └────────┘
```

The requested `301 − 302` state depends only on the links `301 → 399` and `302 → 399`. The `399 → 3 → 399` upstream loop is outside both required paths, so it must not cause an otherwise valid finite relative state to fail. The requested `301 − 0` (SSB) **does** need a connected upstream path: the cycle prevents finding 0 and must still be rejected.

## Small shared-core change

The metadata-only `ancestors` helper now returns a pair:

- **Finite prefix of discovered ancestor IDs:** each body appears at most once, stopping at ID 0, missing coverage or the first repeated ID.
- **Optional repeated ID:** retained for an explicit cycle diagnostic when no shared ancestor was discovered.

`resolve_relative` then finds the first common ID in the target ancestry, using the existing precedence semantics, and evaluates **only** its required state links. If no shared ancestor exists and either path repeats a node, it continues to return the explicit `Cycle { body_id }` error, prioritizing the target's cycle as before. Disconnected acyclic paths remain `Disconnected` errors, and type/frame or invalid coefficient failures on *required* links still fail closed.

This modifies no public API, DAF/SPK format parser, numerical interpolation, units, segment priority, or state source selection. It does not permit a cycle to be traversed during numerical state composition.

## Regressions

Tests use synthetic Type-2 / Type-3 SPK coefficients only; **no real NASA/JPL kernel or third-party code**.

1. **Single kernel:** in both IEEE endiannesses, verify `301 − 399`, `399 − 301` and siblings `301 − 302` resolve accurately despite the irrelevant upstream cycle. Check the nearest common ancestor, XYZ position, and XYZ velocity; `301 − 0` still returns `Cycle { body_id: 399 }`.
2. **Ordered kernel set:** place child-to-parent links in an older kernel and the cycle in a newer kernel, exercising both byte-order permutations. Verify the same correct sibling relative state and preserved root-relative cycle error.
3. Existing tests already cover missing coverage, disconnected branches, required unsupported frames and types, and standard cycles.

## Verification

```sh
cargo fmt --all -- --check
cargo test --workspace spk_relative_state::tests::shared_ancestor_before_cycle_keeps_required_relative_state
cargo test --workspace spk_kernel_set::tests::cross_file_upstream_cycle_does_not_block_earlier_shared_ancestor
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Require the exact PR head to pass the read-only CI, including native Windows and macOS jobs. This is a **center-chain metadata correctness** fix; it does **not** demonstrate independent SPICE/Horizons parity or production accuracy. For real-kernel scientific validation, refer to the repository's separately maintained NAIF reference comparison workflow.
