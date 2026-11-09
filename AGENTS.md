# AI and human coding agents — VirgoStars Ephemeris

This is a **PUBLIC** repository for an independent, originally implemented astronomical engine. The source of truth is its own code/tests, not the private VirgoStars monorepo.

1. Read `README.md`, `docs/ARCHITECTURE.md`, `docs/API_CONTRACTS.md`, `docs/VALIDATION.md`, `docs/DATA_AND_LICENSING.md`, `docs/ROADMAP.md` before changing calculations.
2. **Clean-room implementation only:** DO NOT paste, translate, adapt, port, disassemble, or mechanically reimplement Swiss Ephemeris source or other incompatible/proprietary source. Define behavior from independently usable scientific publications, NAIF format specifications, IAU standards and observable outputs; document provenance for each algorithm. If you have examined restricted implementation code, disclose the risk and request review before implementing the affected algorithm.
3. Treat published astronomical specifications as reference, not a license to copy protected prose/code. Do not copy legacy business/private application logic, production files, third-party fonts, interpretation corpora, user data, keys, API tokens or private paths.
4. Data/license separation: repository MIT applies **only to original code**; NASA/JPL kernels and any other components retain their own terms. No SPK/NAIF Toolkit binaries committed. Validate download source, provenance, checksums, and licensing.
5. Define reference frame, time scale, center, apparent/geometric convention, units, observer, correction model, kernel ID/version and validity range for each numerical result. Never infer missing parameters silently.
6. Reject unsupported epochs, missing segment chains, incompatible kernels, unknown identifiers and precision shortfalls with typed errors. Never silently fall back to an unapproved model.
7. Add synthetic tests and comparison with an **independent astronomical oracle** (JPL Horizons / controlled NAIF tests); log tolerances, uncertainties, algorithm versions, kernel hashes and failure reasons. Performance is secondary to numerical correctness.
8. Document run/test commands **only after implementing them**. Never declare tests green without executing. No automatic deployment, npm publishing, GitHub Actions or changes to VirgoStars production without explicit separate scope.
9. Use feature branches and reviewed PRs. For any public-facing compatibility claim, update the coverage matrix and test evidence.
10. No autonomous long-running tasks, downloads of large files, destructive actions, or releases without deliberate review. No NASA/JPL/NAIF/Astrodienst affiliation or endorsement claims.

Status labels: `DESIGNED`, `IMPLEMENTED_UNVERIFIED`, `VALIDATED_LOCAL`, `RELEASE_CANDIDATE`, `RELEASED`. The initial Node ESM code is `IMPLEMENTED_UNVERIFIED` and is not scientifically validated against real NASA/JPL kernels.
