# Dense Horizons position fits and interleaved holdout

**Correction:** the ancient rotation-fit residuals in the orientation section
below include a significant one-step linearization artifact. A replay of the
same data with iterative rotation composition, plus a wider epoch sample, is
documented in [the frame-fit correction](HORIZONS_FRAME_EPOCH_HOLDOUT_2026-11-18.md).
The old fit numbers remain as historical results, not physical error estimates.

**Status: exploratory validation.** This continues the [near-solar Horizons
rate and frame-fit audit](HORIZONS_SOLAR_RATES_AND_FRAME_ROTATION_2026-11-18.md).
The analytic deflection derivative remains in an offline example; production
calculations, API contracts, stellar data and server integration are unchanged.

## Method and sample

The previous 14 cases across seven bodies were reconstructed with the local
classic DE441 and the existing selection script. The archived prior local
report was unavailable; the reconstructed selection and seven-position
Horizons comparison reproduced its published summary. New SHA-256s below
identify the reconstructed inputs, not the missing original report.

For each case, query Horizons apparent geocentric ICRF quantity 45 at 61
TT instants around the center, spaced 1/256 TDB day after an approximate
one-term TDB→TT conversion. Both implementations use DE441; Horizons is an
independent implementation, not an independent planetary ephemeris. Convert
its printed angular positions to tangent-plane offsets, then fit polynomials
in scaled time using reorthogonalized modified Gram–Schmidt. Differentiate
at the center. Test degrees five and seven on the full ±30/256-day window
and the central ±15/256-day window. Center elongations remain 0.42°–2.26°;
there is no general visibility or no-crossing assertion over the windows.

For a check against overfitting, refit on alternating times and measure the
RMS position residual at the remaining interleaved times. Training and
holdout positions are correlated samples from the same Horizons solution;
their residuals are not independent ephemeris accuracy or a velocity error
bar. The tangent-plane anchor is selected from the training times, including
in the narrow-window case where the center belongs to the holdout. Comparing
degrees and windows is a sensitivity study. Horizons printed
coordinate precision, light-time epoch rounding, the local unrefined single
JD and approximate time-scale conversion remain limitations.

## Sample outcome

Using the full-window degree-five fit as a reference, the candidate has a
smaller tangent-rate residual in 10 of 14 rows; the largest difference among
the four reference fits for a single row is **0.0001492″/day**. Since that
spread can exceed the method difference, neither the count nor any one fit
establishes a general winner.

The most step-sensitive example, Mercury near 2000 CE at 0.416° elongation,
was selected **after** inspecting the results. Its angular-rate residuals
against each fit, in arcseconds/TDB day, are:

| Fit | Current | Candidate | Interleaved holdout position RMS |
| --- | ---: | ---: | ---: |
| Wide degree 5 | 0.00005693 | 0.00014691 | 0.00001070″ |
| Wide degree 7 | 0.00010197 | 0.00000380 | 0.00000154″ |
| Narrow degree 5 | 0.00009498 | 0.00000569 | 0.00000160″ |
| Narrow degree 7 | 0.00009652 | 0.00001657 | 0.00000182″ |

Here the wide degree-five fit predicts held-out positions less well and
gives the opposite rate ranking to the other fits. The fit checks support
further investigation of the candidate for this case, but post-hoc selection,
correlated reference samples and model/rounding effects prevent a validated
accuracy claim or general promotion of the candidate. The solar model's floor
and exact boundary remain unresolved separately.

## Orientation-frame stability across one year

At 13 TT instants spaced 30 days apart in each of two eras (astronomical
years −9998 and 2000), query Horizons for apparent geocentric ICRF and true
equatorial positions of the Sun, Moon, Venus and Mars–Saturn system
barycenters. As in the earlier study, project the same Horizons ICRF
directions into the local experimental mean and true equators, then fit a
first-order three-axis rotation to the six Horizons true-equatorial
directions at each epoch. The fits report exact-rotation residuals and
elimination pivots; six directions do not turn the result into a physical
Earth-pole observation.

| Era | Local equator | Mean fitted rotation magnitude | Max vector deviation from its era mean | Max fit RMS |
| --- | --- | ---: | ---: | ---: |
| −9998 | Mean | 220.575″ | 2.760″ | 0.021″ |
| −9998 | True | 214.389″ | 0.116″ | 0.018″ |
| 2000 | Mean | 16.749″ | 2.483″ | 0.000071″ |
| 2000 | True | 0.05293″ | 0.000805″ | 0.000003″ |

The ancient true-frame discrepancy is largely stable over these 13 samples;
this strengthens its interpretation as a systematic difference in the
orientation conventions. It still combines Horizons' and the local model's
precession, nutation and frame definitions, and one year does not constrain
long-period nutation or establish which model is physically more accurate.
Sampled variation and fit residuals are not uncertainty bounds. The fit's
smallest elimination pivot across these samples is 1.90; pivots are not a
formal condition-number bound.

## Reproduction and local evidence

The kernel and source reports are checked by SHA-256 before and after, and
all scripts require new output paths. Horizons sampling steps require network;
the holdout analysis is offline and revalidates the original response
metadata and epochs. Responses and reports remain local and ignored.

```text
cargo build --offline --locked -p virgostars-ephemeris-core --example compare_deflection_rate_candidates
node bindings/node/scripts/audit-solar-deflection-candidates.cjs <de441> <sha256> <deflection-executable> <new-baseline> baseline
node bindings/node/scripts/compare-horizons-near-sun-rates.cjs <de441> <sha256> <new-baseline> <baseline-sha256> <deflection-executable> <new-prior-report> fine
node bindings/node/scripts/fit-horizons-near-sun-rates.cjs <de441> <sha256> <new-prior-report> <prior-sha256> <deflection-executable> <new-dense-report>
node bindings/node/scripts/audit-horizons-polynomial-holdout.cjs <new-dense-report> <dense-sha256> <new-holdout-report>
cargo build --offline --locked -p virgostars-ephemeris-core --example diagnose_horizons_orientation_components
node bindings/node/scripts/sample-horizons-frame-stability.cjs <orientation-executable> <new-frame-report>
```

- Reconstructed DE441 case report: `60e635a334945fab5704417a0c8d6fb4fc6aec2092504689c87e104e6da7dc52`.
- Reconstructed Horizons seven-position report: `13cb5289b48285eadf5d68a0e674f952fe268b8e30c667fe4f25db2411f5bf17`.
- Dense 61-position report: `62abfbe1f7d3284f4fff3ce131c2c477ab93836ff584c308223b8ea14b48cf37`.
- Interleaved holdout report: `adb04e5f6068b44fdebe07b20902ebfa353f500ec07462e2b40c6964c4bf05ee`.
- Two-era frame-stability report: `405baf381ba1fa8e5f1edff4ff36e66f428ebca51f4685f3b33058f5de213ecd`.
