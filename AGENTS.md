# AGENTS.md — Instructions for AI and human contributors

> Applies to the entire `Cesare-ml/VirgoStars-Ephemeris` repository, including future crates, bindings, tests and documentation. Read this file **before editing**. More specific instructions may be added in subdirectories; they must not contradict the license, scientific integrity or repository-safety rules here.

## 1. Mission and scope

Build a **free, reusable, independently implemented astronomical ephemeris engine** for VirgoStars and third-party software. The canonical implementation is **Rust**, as defined by [ADR-0001](docs/ADR-0001-RUST-CORE-AND-BINDINGS.md). Future JavaScript/TypeScript (WASM) and Python (PyO3/maturin) interfaces shall call the same Rust core instead of duplicating the astronomy.

Scope **inside** this repository: ephemeris file reading, validated interpolation, explicit time scales, celestial body/observer geometry, coordinate reference frames, documented physical corrections, error handling, low-level astronomical events, and a future separately validated no-file *lite* mode.

Scope **outside** this repository:
- `astrological-functions`: zodiac systems, houses, aspects, chart models, interpretation and astrological business logic.
- VirgoStars/Astrolabio and other apps: authentication, databases, GUIs, subscriptions, content, deployments and production migrations.
- `astrological-assets` / `astrological-charts`: artwork, symbols and visual chart rendering.

Do not copy unrelated application code or data into this project. Do not create implicit coupling to private repositories.

## 2. What currently exists — verify the branch

- `main` is the protected integration branch. Its contents are **not necessarily the same** as active development branches.
- Rust work is being developed on `dev/ephemeris` and/or feature branches. Inspect the target branch first; never assume an ADR's proposed folder structure has been implemented.
- Some historical code is experimental JavaScript (including a classic JPL DE441 reader). It is a prototype and is **not** a second production source of astronomical truth.
- The experimental Rust workspace, when present, contains `Cargo.toml`, `crates/ephemeris-core/`, and `docs/DE441.md`. Its DE441 reader supports the **classic binary** format, not necessarily NAIF SPK `.bsp` files. Confirm actual supported formats in code.
- README / ADR / planning documents can lag behind implementation. Correct contradictory documentation in a focused PR; do not claim an unmerged change is on `main`.

Never report SPK compatibility, JPL/Horizons validation, WASM/Python binding availability, numerical parity, production readiness or package publication without corresponding checked evidence.

## 3. Required reading before making changes

1. `README.md` and this `AGENTS.md`.
2. `LICENSE`, `NOTICE` and `docs/LICENSING_AND_CREDITS.md`.
3. `docs/ADR-0001-RUST-CORE-AND-BINDINGS.md`.
4. Any relevant actual module, tests, data provenance notes and existing PRs/commits on the branch being edited.
5. `docs/DE441.md` when working on the Rust classic-DE441 reader (if present).

A broken or missing link is not proof that the file exists. Check the repository tree before citing documentation or implementing a design.

## 4. Git workflow and preservation of user work

- **Never push directly to `main`, force-push `main`, disable branch protection or bypass review.**
- For scoped work, start from a clean, up-to-date branch and create a descriptive feature/docs/fix branch. Use `dev/ephemeris` as the active development branch when appropriate, but check its current state and any pending PRs before basing work on it.
- Small, focused commits with descriptive messages; open a PR to the appropriate target branch. Explain scope, risks, tests and missing validation.
- **Do not merge an unrelated or unfinished PR**, switch base branches casually, delete branches, rewrite history or publish packages without the owner's authorization.
- Before any checkout, reset, rebase, merge, cherry-pick or stash operation, inspect `git status`. Preserve uncommitted/staged changes, untracked files and the user's work.
- If the Git index has unmerged paths, **stop normal development first**. Inspect `git status` and conflict stages; resolve conflicts deliberately or ask for a decision. Never choose `ours`/`theirs` indiscriminately.
- Do not stage or commit conflict markers: `<<<<<<<`, `=======`, `>>>>>>>` (except deliberate test fixtures). Check modified text files and `git diff --check`.
- Never solve a Git problem using `git reset --hard`, `git clean -fd`, force push or blanket “discard changes” without explicit authorization and a verified backup.
- Prefer additive, isolated improvements and reversible migrations. Do not silently change VirgoStars/Astrolabio production or other repositories.
- **Do not introduce GitHub Actions** or automatic deployment/release workflows. Local/manual tests are the default until the owner explicitly changes that decision.

Suggested local checks:
```bash
git status
git diff --check
git grep -n -E '^(<<<<<<< |=======|>>>>>>> )' -- . ':!AGENTS.md'
```
The last command is a *guard*: a nonzero code means “no matches” or another Git error; inspect output/context. It is not a replacement for reviewing merge state.

## 5. Architecture and implementation rules

- Rust is the **single authoritative astronomical core**. Keep I/O, parsing, interpolation, time-scale conversion, transforms and correction stages modular and independently testable.
- JavaScript/TypeScript/WASM and Python bindings must stay thin; do not recreate ephemeris arithmetic in another language. Expose consistent versioned contracts, explicit errors and equivalent options.
- No hidden network calls, automatic kernel downloads, global data paths or runtime dependence on an app's database.
- Prefer explicit parameters and types to magic defaults. Document file formats, coverage intervals, time scale and limitations.
- Avoid breaking public APIs without a versioned change, migration instructions and tests. Distinguish proposed APIs from shipping APIs.
- Performance matters, but correctness, traceability, portability and numerical stability take precedence over micro-optimizations.
- Keep the core independent of Angular, authentication, subscriptions and interpretations.

## 6. Scientific accuracy and validation — non-negotiable

Every reported result must specify, where relevant:
- **Epoch/time scale:** e.g. Julian Date TDB, UTC, TT; conversions and leap-second assumptions.
- **Target and center/observer:** planet center vs system barycenter, geocentric/topocentric/barycentric.
- **Frame/equinox:** e.g. ICRF/J2000 or ecliptic/equator of date, and which transformation conventions were applied.
- **Quantity and corrections:** geometric vs astrometric vs apparent; light-time, aberration, deflection, precession, nutation, etc., if present.
- **Units:** e.g. km, km/day, radians/degrees and time units.
- **Range, failure modes and precision/uncertainty** appropriate to implemented algorithms and datasets.

Tests should cover known fixtures, boundaries, invalid/short/corrupt files, off-by-one records, integer/float safety, Earth–Moon/barycenter handling and deterministic errors. Keep regression tests and truly **independent** scientific validation separate.

Do not use comparisons against this repository's previous JavaScript results as proof of JPL accuracy. Obtain independently traceable references (e.g. JPL Horizons / NAIF, with query settings and dates), document tolerances and fixtures, and note when a test is unexecuted or data unavailable. A synthetic fixture proves parser behavior, **not** real-world numerical parity.

Mark approximate and *lite* modes clearly; never silently fall back from high-precision ephemeris files to lower-precision calculations. Do not label geometric results “apparent”.

## 7. Local build and test protocol

For a branch that includes the Rust workspace:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

A separate optional integration check, **only when the external file is available**:

```bash
# Configure DE441_PATH for your shell to point to a legally obtained classic DE441 file.
cargo test --workspace -- --ignored
```

Do not assume `cargo`, Clippy, network access, or external binaries are installed; report missing prerequisites accurately. Run tests appropriate to the changed scope. If the branch still contains JavaScript prototype tests, check its actual runtime and commands before running them. Never state “tests passed” unless you executed them and saw success. Record commands, platform, results, skipped tests and known limitations in the PR.

## 8. Licensing, attribution and third-party materials

- **Repository source code license: CPAL-1.0**, with completed Exhibits A/B in `LICENSE`; read `NOTICE` and `docs/LICENSING_AND_CREDITS.md`. Do **not** describe the repository as MIT-licensed or unlicensed.
- Preserve required source headers/notices and applicable change/date records. When introducing/modifying covered files, follow CPAL's notice and modification-documentation requirements; avoid inventing rights or owners.
- The applicable VirgoStars attribution includes **“Powered by VirgoStars”** and **https://www.virgostars.it/**. In graphical consumers, follow the actual CPAL §14 and Exhibit B prominence/launch-or-session requirements; an About/Credits link alone is not guaranteed sufficient. Review CPAL §15 for applicable network deployments.
- No charge for access to the official source code and no royalties claimed by the project, but commercial reuse remains subject to the actual license terms.
- Treat all external code, datasets, graphics and binaries as **separately licensed**. Do not copy/translate Swiss Ephemeris or Astrodienst code, vendor NAIF SPICE Toolkit code, or redistribute JPL ephemeris kernels/`.eph`/`.bsp`/`.se1` files without a documented rights/provenance review. Public availability is not blanket permission to relicense under CPAL.
- Cite scientific publications and algorithm specifications; include third-party notices and compatible licenses for any permitted dependency.
- Do not alter license texts, Exhibit B attribution, NOTICE or license strategy incidentally to a code task. Escalate uncertain provenance or legal compatibility before adding material.

## 9. Review checklist for every agent task

Before changing files:
1. Identify the exact branch, clean/dirty status, user goal and allowed paths.
2. Read applicable docs, current implementation and adjacent tests.
3. Define expected behavior, units, error cases and a minimal validation plan.

Before proposing a PR:
1. Inspect the diff for unrelated edits, secrets, large data files and conflict markers.
2. Add/update tests and explain any independent-validation gap.
3. Update accurate documentation and licensing notices where required.
4. Run available relevant checks; state what was *not* run and why.
5. Summarize changes, risk, compatibility, branch/PR link and next actions.

**Owner approval is required** before publishing packages, changing repository visibility, changing licensing, adding GitHub Actions, integrating into production, deleting user data or performing destructive repository operations.

## 10. Reporting format

Use a compact handoff note or PR description containing:
- **Scope:** goal, base/head branch, files changed.
- **Scientific contract:** time scales, frames, centers, units and corrections affected.
- **Verification:** exact commands + results; synthetic vs independent reference coverage.
- **License/provenance:** dependencies or third-party data touched.
- **Risk/next steps:** known blockers; what still requires owner review.

Communicate in Italian when working directly with the owner; code and repository documentation can remain in clear technical English. Never present planned work as delivered work.

---

This guide describes development behavior; the checked-in `LICENSE` controls actual legal rights and obligations. When repository state, scientific accuracy or an instruction conflicts with safety/integrity, stop the unsafe action, report the conflict, and propose a reversible solution.
