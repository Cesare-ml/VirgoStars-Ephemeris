# Branch integration status and scientific release gates — 2026-11-10

This is a dated engineering snapshot, **not a claim of a released or validated ephemeris**. Always inspect the live branches and PRs before acting: this page can become stale.

## Canonical decisions

- The original repository code is licensed under **CPAL-1.0**, with Exhibits A/B, [NOTICE](../NOTICE) and [required VirgoStars credits](LICENSING_AND_CREDITS.md). Preserve third-party ERFA/NumFOCUS notices and the separate rights of NASA/JPL datasets.
- **Rust is the one intended authoritative astronomical engine** ([ADR-0001](ADR-0001-RUST-CORE-AND-BINDINGS.md)). JavaScript/TypeScript via WASM and Python via PyO3 are *planned thin bindings*, not parallel astronomy engines.
- The original JavaScript in `main` is experimental research and a source of regression ideas; passing its synthetic/ERFA tests does not make it a Rust-backed shipping package.
- Do not merge incomplete scientific functionality into a production application, distribute unpublished kernel files, silently enable approximate fallbacks or claim Swiss Ephemeris parity.

## Verified branch distinction

| Branch | Snapshot |
| --- | --- |
| `main` | CPAL-1.0 license, AGENTS.md, research JavaScript SPK/IAU examples, tests, documentation and CI. **No Cargo workspace or shipping Rust implementation** at the time of this snapshot. |
| `dev/ephemeris` | Experimental Cargo workspace with a **classic JPL DE441 binary** reader, interpolation, numerical primitives and synthetic/integration regression tests. This reader does **not** parse NAIF SPK `.bsp` files. |
| Feature / historic PR branches | May contain obsolete or stacked implementations. Evaluate diff, base and duplicate work before merging. |

In development, [PR #22](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/22) integrated the numerical foundation that superseded overlapping [PR #8](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/8) and [PR #14](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/14).
[PR #26](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/26) added proleptic calendar and split Julian-date conversions and was merged into `dev/ephemeris` on 2026-11-10.
[PR #27](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/27) added synthetic finite-difference velocity consistency tests and was merged into `dev/ephemeris` the same day.

These **development-branch merges are not merges to `main`**. They do not prove independent NASA/JPL numerical accuracy. Historical [PR #2](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/2) describes licensing as undecided; the canonical decision is already recorded in `LICENSE`, `NOTICE` and `docs/LICENSING_AND_CREDITS.md`.

## Remaining gates for the Rust integration PR to `main`

1. **Reconcile diverged Git history deliberately.** Preserve `main`'s research code, third-party notices, AGENTS.md, CPAL license and licensing guidance while adding the Rust workspace. Do not resolve README/ADR conflicts by indiscriminate `ours`/`theirs`, rewrite `main`, or drop tests.
2. **Review scope and provenance.** Confirm `Cargo.lock`, `.gitignore`, ERFA notices and tests. Generated `target/` artifacts and external `.bsp`/`.eph` data must not be committed.
3. **Run reproducible gates for the exact integration head.** At a minimum `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo test --workspace --release --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `npm test`, and checks for conflict markers/diff whitespace. Record platform, toolchain, precise SHA and results. Existing CI from another branch or commit does not certify the integration commit.
4. **Independent scientific validation is separate.** Real official JPL kernel and comparable NAIF/Horizons reference states, frames, epochs, tolerances and hashes are still an open gate ([issue #3](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/3)). Experimental SPK frame/segment handling and blank-field validation need resolution ([issue #11](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/11)).
5. **Do not deploy or publish** or switch the VirgoStars application merely because a PR merged to `main`. Production adoption requires separately approved validation.

See [AGENTS.md](../AGENTS.md), [ROADMAP](ROADMAP.md), [VALIDATION](VALIDATION.md), [DE441 development documentation](https://github.com/Cesare-ml/VirgoStars-Ephemeris/blob/dev/ephemeris/docs/DE441.md) and [issue #12](https://github.com/Cesare-ml/VirgoStars-Ephemeris/issues/12) for the reconciliation task.

## Documentation change log (CPAL §3.3)

2026-11-10: Corrected outdated references to MIT licensing and the prohibition on CI; aligned contributor guidance and the Italian overview with CPAL-1.0, Rust as the canonical core, and the separation between `main` and `dev/ephemeris`. No scientific code, user data, external datasets, or production deployment was changed.
