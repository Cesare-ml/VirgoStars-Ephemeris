# Contributing

Thank you for helping build an independent astronomical engine.

## Workflow

- Start from an issue describing a single calculation, scientific reference, desired output contract and reproducible test.
- Read [AGENTS.md](AGENTS.md), [ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md) and the applicable roadmap first. Work on a feature branch. Propose a PR with implementation, numerical tests, docs and provenance entries; ask for review before merge. Never push directly to `main`.
- Keep dependencies minimal, compatible with CPAL-1.0 and declared. A new dependency or bundled dataset requires a license audit.
- Do not open PRs with Swiss Ephemeris source, ports of Swiss algorithms, private VirgoStars code, historical databases, third-party imagery, or raw NASA SPK blobs.
- Do not publish novel claims of accuracy until the stated tolerances are measured against independent references.
- Test with synthetic examples and non-identifying times/places; no production birth records.
- Read-only GitHub Actions CI was authorized by the owner on 2026-11-10 for formatting, linting, unit tests and builds, with least-privilege permissions and no deployment or publishing. Also record native/local verification and independent astronomy validation separately; passing CI alone is not scientific certification.
- Original contributions to the covered code must be submitted under the repository's current **CPAL-1.0** terms, retaining Exhibit A notices and documenting dated modifications under Section 3.3. Contributors must have rights to their contributions; third-party works retain their own terms and notices. Record scientific citations and provenance. If licensing is uncertain, stop for human review.

## PR checklist

- [ ] Scope / status label and API semver impact recorded
- [ ] Scientific source(s), kernels, frames and corrections documented
- [ ] Independent numeric reference and thresholds attached
- [ ] Boundary/edge cases (time, 360° wrap, missing kernels, ranges) tested
- [ ] CPAL-1.0 Exhibit A/change notices, third-party licenses and dataset attribution reviewed
- [ ] No secrets, personal records, proprietary sources, generated binaries
- [ ] No production integration or deployment implied
