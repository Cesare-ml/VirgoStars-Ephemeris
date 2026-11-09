# Contributing

Thank you for helping build an independent astronomical engine.

## Workflow

- Start from an issue describing a single calculation, scientific reference, desired output contract and reproducible test.
- Work on a feature branch. Propose a PR with implementation, numerical tests, docs and provenance entries; ask for review before merge.
- Keep dependencies minimal, permissively licensed and declared. A new dependency or bundled dataset requires a license audit.
- Do not open PRs with Swiss Ephemeris source, ports of Swiss algorithms, private VirgoStars code, historical databases, third-party imagery, or raw NASA SPK blobs.
- Do not publish novel claims of accuracy until the stated tolerances are measured against independent references.
- Test with synthetic examples and non-identifying times/places; no production birth records.
- No GitHub Actions are required or requested for this project; validation can run locally and be recorded in PR descriptions.
- Contributions should be provided under MIT for original new code; contributors must have the rights to submit them. Record scientific citations and third-party provenance in documentation. If licensing is uncertain, stop for human review.

## PR checklist

- [ ] Scope / status label and API semver impact recorded
- [ ] Scientific source(s), kernels, frames and corrections documented
- [ ] Independent numeric reference and thresholds attached
- [ ] Boundary/edge cases (time, 360° wrap, missing kernels, ranges) tested
- [ ] Dependencies, dataset attribution and licenses reviewed
- [ ] No secrets, personal records, proprietary sources, generated binaries
- [ ] No production integration or deployment implied
