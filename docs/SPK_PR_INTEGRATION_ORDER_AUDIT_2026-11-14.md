# Read-only audit of the experimental SPK PR integration order

Created **2026-11-14**. The Rust SPK implementation and independent NAIF
comparison are currently split across multiple **unmerged** GitHub PRs.
A passing synthetic CI run on one stacked branch must **never** be mistaken
for a merged, reviewed, independently scientifically validated engine.

## Supported integration chain

The script `scripts/audit_spk_pr_dependencies.py` checks this ordered route
into `dev/ephemeris`, from the foundational DAF/SPK index to reference
vectors and an auditable comparison receipt:

| PR | Subject | Direct prerequisite |
|---:|---|---:|
| #148 | Read DAF/SPK segment directory | None |
| #150 | Evaluate SPK type-2/type-3 direct states | #148 |
| #157 | Resolve single-kernel center chains | #150 |
| #159 | Resolve multi-kernel priority and center chains | #157 |
| #160 | Compare geometric Rust vectors with external reference CSV | #159 |
| #162 | Generate independent NAIF/SPICE reference vectors | #160 |
| #164 | One-command NAIF/Rust comparison receipt | #162 |
| #165 | Independently audit Sun/Moon/Earth/planet reference breadth | Independent |

The manifest deliberately does **not** count partly overlapping #161 or
#163 as additional mandatory stages. They are alternatives/add-ons requiring
human consolidation rather than automatic adoption. Future PR replacements
must be deliberately reviewed and the manifest revised; a cherry-picked
commit does **not** automatically count as a GitHub-merged PR.

## How to run

The live command makes **read-only** public GitHub REST GET requests (one per
listed PR), with an optional `GITHUB_TOKEN` environment variable for
rate-limit headroom. It never stores or prints the token, writes to GitHub,
merges PRs, downloads astronomical kernels, or invokes privileged CI.

```sh
python3 scripts/audit_spk_pr_dependencies.py --live
python3 scripts/audit_spk_pr_dependencies.py --live --json
python3 scripts/audit_spk_pr_dependencies.py --live --require-integrated
```

For offline/CI regression inspection, supply an explicit snapshot keyed by
string PR numbers. Each value must contain GitHub-compatible fields
`number` (integer), `state` (`open`/`closed`), `merged` (boolean),
`draft` (boolean), and `base.ref` (string):

```sh
python3 scripts/audit_spk_pr_dependencies.py --snapshot /path/to/pr-status.json --json
python3 -m unittest scripts.tests.test_spk_pr_dependency_audit
```

An offline snapshot is **caller supplied and not authenticated**. It is
useful for reproducible regression checks, not current GitHub proof. Use
`--live` to query actual status, and independently inspect each PR's **exact
head SHA, latest CI, reviews, merge conflicts, security/data rights and
dependent changes** before integration.

## Status and exit contract

- `MERGED`: the PR is reported merged into `dev/ephemeris` and its listed
  prerequisites are also merged into that branch. This is a GitHub metadata
  check, **not** an audit of equivalence or numerical accuracy.
- `BLOCKED`: at least one prerequisite has not been merged.
- `DRAFT_REVIEW`: dependencies merged, but PR still draft.
- `HUMAN_REVIEW_REQUIRED`: dependencies merged and PR open/non-draft.
  **Never** interpret as automatic merge permission or green CI.
- `CLOSED_UNMERGED`: PR is closed without a merge.
- `BASE_REVIEW`: the PR targets a branch other than `dev/ephemeris`.
- `ORDER_CONFLICT`: a PR was marked merged while a prerequisite was not.
  This may require human reconciliation of a cherry-pick or previous merge.

Default exit is `0` for a valid status report even with blocked/draft work,
`1` if an order conflict exists, `2` for missing/malformed or inaccessible
status input. With `--require-integrated`, exit `1` until **all eight**
manifest PRs report `MERGED` without conflicts or wrong-base metadata.

## Scientific boundary and ownership

Even a complete dependency audit says **nothing** about numerical agreement
with actual NASA/JPL kernels. E1 validation remains gated on authentic SPK
provenance, independent NAIF/SPICE or Horizons vectors, correct target/observer
identities (distinguish Mars barycenter `4` from Mars center `499`),
the `J2000` ICRF-style frame, `TDB` ET seconds past J2000, geometric
`NONE` corrections, **km and km/s**, approved tolerances, proper Sun/Moon/
Earth/planet coverage, and a manually reviewed residual receipt. The official
Horizons VECTORS API explicitly distinguishes `VEC_CORR=NONE` from light-time
and aberrated outputs:
https://ssd-api.jpl.nasa.gov/doc/horizons.html

The GitHub Actions job under `dev/ephemeris` discovers only the **offline**
`unittest` fixtures. It must not call the live GitHub API or merge anything.
No change to Rust astronomy, official JPL data, production, package publication
or releases. CPAL-1.0 notices and **Powered by VirgoStars** attribution remain
mandatory.
