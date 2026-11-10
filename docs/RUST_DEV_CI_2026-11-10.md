# Rust CI gate for the development branch — 2026-11-10

The owner explicitly authorized GitHub Actions for this public repository on 2026-11-10. This PR adds a dedicated Rust gate **on `dev/ephemeris`**, which already contains the Cargo workspace. The separate PR #13 targets `main` and has conditional Rust steps that currently skip while Cargo is absent there.

- Workflow: `.github/workflows/rust-dev.yml`.
- Scope: Rust changes to `dev/ephemeris`, its pull requests and manual dispatch.
- Gates: `cargo fmt --all -- --check`, `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- Cross-platform compile check: install the `x86_64-pc-windows-gnu` Rust target and run `cargo check --workspace --locked --target x86_64-pc-windows-gnu` on Ubuntu. This checks Windows-target Rust type checking without executing Windows binaries or performing a full native Windows linker/build; it does not prove Windows runtime compatibility.
- **Native Windows runner added 2026-11-10:** on `windows-latest`, execute `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings`. Unlike cross-compilation, these run the unit/integration test binaries on Windows. Passing tests cover the Rust workspace's tested behavior only, not external DE441 kernels or every supported Windows version.
- Security: read-only repository token; no secrets, artifacts, third-party data downloads, automatic merges, deployments or package publication.
- Rust tests are synthetic or local regression tests; they **do not establish** agreement with NASA/JPL kernels or Horizons.
- Coordinate this gate with PR #13 during integration to avoid permanently duplicating Rust CI jobs on `main` and `dev/ephemeris`.
- Scientific contract unchanged: no changes to frames, timescales, units, bodies, corrections or numerical methods.
- CPAL-1.0 and the VirgoStars attribution remain unchanged; workflow includes Exhibit A notice.

## CI modification history

- **2026-11-10:** Added a separate Windows-hosted test/Clippy job to complement the existing Ubuntu suite and Windows GNU cross-target check. This is an incremental CI change and does not modify scientific algorithms or third-party data. Verified successful in GitHub Actions run #42782149595 (both Ubuntu and native Windows jobs completed successfully).

- **2026-11-10:** Added a macOS native job using `macos-latest` to execute locked Rust workspace tests and strict Clippy. This checks native macOS behavior without introducing additional crates or external data; the job's result must be checked separately before declaring it passed.

- **2026-11-10:** Added an Ubuntu Rustdoc gate: `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`. This checks that the public Rust API documentation builds with warnings treated as errors, without pulling dependencies or generating published documentation. It is not a scientific correctness test. Verified results must be read from the corresponding GitHub Actions run.
