# CI authorization and implementation — 2026-11-10

This change was made in a dedicated feature branch and proposed for review rather than pushed directly to `main`.

- The repository owner explicitly authorized standard GitHub Actions in the public VirgoStars-Ephemeris repository on 2026-11-10.
- Added `.github/workflows/ci.yml`: Node.js 22 synthetic/prototype regression tests, and conditional Rust format, unit test and Clippy checks when `Cargo.toml` is present.
- Changed `AGENTS.md` and `docs/ROADMAP.md` to reflect that approval while retaining explicit restrictions on automatic releases, deployments, data downloads and package publication.
- Fixed the outdated MIT reference in the E0 exit gate; the current source-code license remains CPAL-1.0.
- The `README.md` status on this PR branch describes the proposed CI. It is not yet a claim that `main` has been updated.
- No astronomical algorithms, reference kernels, license terms or attribution were changed. CI does not prove numerical agreement with real NASA/JPL data.

Scientific contract: unchanged. No modification to body centers, reference frames, timescales, units, physical corrections or uncertainties.

Verification to record in PR: workflow execution and checks after GitHub accepts this branch; local Cargo is unavailable in the initiating environment.
