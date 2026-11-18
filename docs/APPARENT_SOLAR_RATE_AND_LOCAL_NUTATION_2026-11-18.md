# Apparent ICRF solar-rate and local-nutation diagnostics

**Status: research only.** Continuation of the [Horizons equator and solar
stencil audit](HORIZONS_EQUATOR_AND_DEFLECTION_STENCILS_2026-11-18.md).
The candidate is an offline example, not a production or public API change.
Stars and main-server migration remain paused.

Continuation: [Horizons sampled speeds and equatorial frame fit](HORIZONS_SOLAR_RATES_AND_FRAME_ROTATION_2026-11-18.md).

## Apparent ICRF speed through aberration

For the existing real DE441 grids (121 rows / 105 distinct body-epoch pairs,
and 242 rows / 210 distinct), propagate both the existing and analytic
deflection velocities through the same light-time and aberration stages.
The candidate uses the existing deflected position, differentiates its
normalized direction analytically and includes range rate; the diagnostic
checks that the two candidate positions agree. These use the unrefined
single-f64 emission JD, not the separate emission-refinement experiment.

The **full** ICRF unit-vector rates below are compared to fourth-order
finite differences of the same apparent positions. Reference half-steps are
1/512 and 1/1024 TDB day. The unit is arcseconds/TDB day; maxima can belong
to different samples, and results include occulted lines of sight.

| Grid | Current max residual | Candidate max residual | Reference-step max disagreement | Candidate smaller |
| --- | ---: | ---: | ---: | ---: |
| Initial | 0.0015195 | 0.0005174 | 0.0017023 | 60/121 |
| Offset | 0.0386751 | 0.0005573 | 0.0010558 | 115/242 |

The full-position derivative can be noisier than the candidate residual;
the table cannot establish a general accuracy improvement. It tests internal
consistency, not an independent apparent-speed reference.

An alternate diagnostic differentiates the small **difference** between
apparent directions with and without solar deflection, both including
aberration. This reduces common-mode numerical cancellation but is still a
same-model comparison. Reference half-steps are 1/2048 and 1/4096 day:

| Grid | Current max | Candidate max | Reference-step max disagreement | Candidate smaller |
| --- | ---: | ---: | ---: | ---: |
| Initial | 0.0015277 | 0.000001570 | 0.000000985 | 65/121 |
| Offset | 0.0386710 | 0.000001538 | 0.000001481 | 125/242 |

Near the numerical reference's own precision, per-row ranking is unreliable;
none of these finite-difference samples crossed the denominator floor at
the sampled times. A separate ignored test, executed locally with DE441,
checks one near-solar Jupiter-barycenter case: candidate and current
positions agree, the two full-pipeline reference steps agree to within 5%
of the current residual, and the candidate residual is less than 5% of it.
This is one internal numerical regression, not a physical accuracy bound.
Guard crossings and physical interpretation of occulted cases remain open.

## Local nutation contribution against Horizons

Reuse the earlier three-body, seven-epoch Horizons responses, validating the
original raw target, reference and TT metadata. Project **the same Horizons
ICRF direction** through the local mean equator and, separately, through the
local true equator with experimental extrapolated nutation. Compare both with
Horizons' apparent **true** equator of date. This is an isolation experiment,
not a comparison of two independently known mean equators.

At astronomical year −9998, the Sun's angular separation from Horizons'
true equator result is **218.52″** with the local mean equator and **214.44″**
with the local true equator; the local nutation changes the direction by
**8.65″**. Thus toggling local nutation alone does not remove the discrepancy
in this sample. The mean-versus-Horizons comparison still includes Horizons'
own nutation and reference conventions; it is **not** a pure precession
error. It neither determines which model is physically better nor bounds
accuracy at other times.

## Reproduction and local evidence

The input reports and kernel are checked against caller-provided SHA-256s
before and after; reports remain local and ignored. Full-rate reports were
recorded before the script added an explicit `mode` field, so their archived
hashes identify those exact files, not byte-identical newer replays.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
cargo test --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates real_de441_apparent_rate_candidate_is_consistent_for_sampled_near_sun_case -- --ignored
node bindings/node/scripts/audit-apparent-icrf-solar-rate.cjs <de441> <sha256> <prior-case-report> <prior-sha256> <deflection-executable> <new-report> full
node bindings/node/scripts/audit-apparent-icrf-solar-rate.cjs <de441> <sha256> <prior-case-report> <prior-sha256> <deflection-executable> <new-report> increment
node bindings/node/scripts/analyze-horizons-nutation-contribution.cjs <orientation-report> <sha256> <projection-executable> <new-report>
```

Set `DE441_PATH` before the ignored test. SHA-256s of the five local reports:

- Full initial: `4440ae1f6bbbd63146dcf691d03cceaa095409f29511d97530ed0a204c071a4c`.
- Full offset: `2377a90df31cdf9acb44788adc5456361c2d6e874f0c816628c1da55f0b382f9`.
- Deflection increment initial: `c4686e64a56427d056a1fca014caa2243214562cef9e72e0e9c2bcce7531f511`.
- Deflection increment offset: `76ca93b0b14e13191618aa6ead61a037cdf3774fdd3774bf0aaa7b1de1540e60`.
- Local nutation contribution: `02836335b3a7db8c8bb0677b5791e4cb5230d73c10529c359b9361edbc00ee46`.
