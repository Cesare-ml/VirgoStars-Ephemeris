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
`draft` (boolean), `base.ref` (string), and `merged_at` (timezone-aware ISO 8601 timestamp when merged; null otherwise):

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

- `MERGED`: GitHub reports this PR merged into `dev/ephemeris`; each
  prerequisite is validly merged **strictly earlier** according to
  timezone-aware `merged_at` timestamps. This remains a GitHub metadata
  check, **not** proof of integrated source equivalence or accuracy.
- `BLOCKED`: at least one prerequisite is not **validly merged into
  `dev/ephemeris`**. A GitHub-merged parent with a wrong base or an
  earlier out-of-order merge **must not** unlock its descendants.
- `DRAFT_REVIEW`: dependencies merged, but PR still draft.
- `HUMAN_REVIEW_REQUIRED`: dependencies merged and PR open/non-draft.
  **Never** interpret as automatic merge permission or green CI.
- `CLOSED_UNMERGED`: PR is closed without a merge.
- `BASE_REVIEW`: the PR targets a branch other than `dev/ephemeris`.
- `ORDER_CONFLICT`: a PR was marked merged while a prerequisite lacks
  a valid `MERGED` audit status, or its recorded merge timestamp is
  **strictly earlier** than the prerequisite's timestamp. The conflict
  propagates transitively and requires human reconciliation.
- `ORDER_UNVERIFIED`: a merged PR and a direct prerequisite share the
  same recorded merge instant. The order cannot be proven from GitHub
  timestamp precision alone; do not treat it as merged safely.

Default exit is `0` for a valid status report even with blocked/draft work,
`1` if an out-of-order merge **or a merged PR with the wrong target branch**
exists (or merge chronology cannot be established), `2` for missing,
malformed or inaccessible status input. With `--require-integrated`, exit `1` until **all eight**
manifest PRs report `MERGED` without conflicts or wrong-base metadata.

### Merge timestamp verification

`merged_at` is mandatory and timezone-aware for every PR marked merged.
A missing, malformed, timezone-naive merge timestamp (or a non-null timestamp
for an unmerged PR) is rejected with exit code `2`. The auditor normalizes
UTC offsets before comparing instants: `12:00:00Z` and `13:00:00+01:00`
are identical. An identical-second merge pair is classified as
`ORDER_UNVERIFIED` and exits `1` (never silently treated as safe). The
JSON report also includes each PR's original `merged_at` field.

This is **conservative merge metadata checking**, not a Git commit-history
or tree-equivalence proof: squashes, cherry-picks, and subsequent changes
still require manual inspection. Offline snapshots are untrusted.

## Next review stage (advisory only)

The report now includes `next_review_stage` in JSON (or `NEXT HUMAN REVIEW`
in console output). This selects the first chain item that is unmerged but
has successfully audited predecessors and is either `DRAFT_REVIEW` or
`HUMAN_REVIEW_REQUIRED`. It does **not** inspect GitHub CI, approvals,
security, science receipts, current mergeability or branch conflicts and
**never** authorizes a merge. `null` means no stage is currently ready
for a human review, which may mean either complete integration or blocking
errors; inspect the detailed statuses to tell which.

`blocking_prerequisites` lists direct ancestors whose audit status is not
`MERGED`, including ancestors nominally merged on GitHub but marked
`BASE_REVIEW` or `ORDER_CONFLICT`. The pre-existing
`unmerged_prerequisites` field is retained for consumers that need to
distinguish GitHub's raw merge flag from safe dependency readiness.

### Checked live snapshot — 2026-11-14

The read-only GitHub inspection on **2026-11-14** found that
#148, #150, #157, #159, and **#160** were merged into `dev/ephemeris`.
The observed `merged_at` timestamps (UTC) increased in dependency order:
`12:58:06`, `13:04:58`, `13:09:45`, `13:13:53`, and `13:19:10`.
The next sequential step is **draft PR #162** (independent NAIF/SPICE
reference generation), with #164 still blocked behind it; #165 remains
an independent reference-breadth audit. This observation confirms
**metadata order only**, not whether any source tree or kernel has been
independently validated. It is a dated snapshot, not live status:
rerun `--live` and review exact heads and CI before any integration.

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
