# DE441: lunar regression and independent rate follow-up — 2026-11-18

**Status: exploratory.** This follows the [independent audit](DE441_INDEPENDENT_VALIDATION_2026-11-18.md). It does not activate an app engine or establish a production accuracy budget. All tests here use the same DE441 solution in two independently implemented programs; they do not independently establish the physical accuracy or provenance of the orbit itself.

## Historical and versioned lunar regression

The previous audit found that the original six-field same-engine regression passed at `6835e00e4b4ba5fc52d882a793e2dfa2b2a436aa`, and failed after the light-time convergence change at `ff26f5ab0bcfaed0f0b8b76fbf5c2392de5be400`: 137 of 1,990 rows exceeded at least one existing tolerance, all for the Moon. The old fixture remains unchanged; its Git blob ID is `b7a232a02e0659407df2d4a5a71dafd031f58c00`.

Before obtaining additional data, the local plan fixed a 0.05 arcsecond diagnostic direction limit for each version, and a 0.00001 arcsecond neutral reporting margin. On all **199** historical lunar epochs, Horizons Q45 supplied apparent, airless, geocentric ICRF directions for Moon 301 relative to Earth 399; the core's standard apparent true-equatorial results were rotated back to ICRF with ERFA IAU 2006/2000B. The epochs use JD TDB in the engine and approximately converted JD TT in Horizons. Both use DE441 and solar deflection, aberration and light time. At the 137 changed rows the historical implementation's largest direction residual was **0.00131039″**, versus **0.000293984″** for the current one: all 137 current residuals improved by more than the reporting margin. Across all 199, maximum residuals were **0.00131039″** and **0.000868977″** respectively; 137 improved, zero worsened, and 62 were neutral within that margin. Neither version exceeded 0.05″. These directional results do not validate all six components, all dates or the absolute ephemeris.

The test now reads the untouched historical fixture for the other nine targets and `evidence/mvp1-apparent-moon-baseline-2026-11-18.csv` for the same 199 lunar epochs. The new fixture was generated with `apparent_six_fields` from source `e65660ee1d22f3e3611461d1f0e771080fb45291`, using the kernel below and the historical lunar TDB dates. It is a **same-engine regression snapshot**, not a JPL reference. The test rejects duplicates, missing or unused lunar dates, checks 1,990 historical and 199 lunar rows, and retains the previous absolute thresholds: 1e-9 degrees or degrees/TDB day and 1e-12 AU or AU/TDB day. The snapshot accepts the reviewed convergence change as a new regression point; the limited directional comparison alone does not independently validate its distance or speed fields.

With the local kernel, `cargo test --release --offline -p virgostars-ephemeris-core --test apparent_ecliptic_of_date -- --ignored` passed **4/4**, including the revised regression. The original failure reported in the previous audit remains the correct historical result on the original commit.

## Experimental ICRF rates: discrepancy remains open

On 61 July 1 epochs at 50-year intervals from astronomical year 0 to 3000 and ten targets (Sun, Moon, Mercury, Venus, Mars–Pluto system barycenters), compare the experimental refined apparent ICRF state with Horizons geocentric airless ICRF. Rates are in arcseconds/hour for RA·cos(dec) and declination. Horizons' [Q46 definition](https://ssd.jpl.nasa.gov/horizons/manual.html) includes light time, gravitational light deflection and stellar aberration. Epoch conversion uses the scripts' approximate one-term TDB-to-TT conversion. Thresholds below were set before fetching and are diagnostic, not uncertainties.

| Reference | Limit per component | Cases above limit | Maximum absolute RA·cos(dec) | Maximum absolute dec |
| --- | ---: | ---: | ---: | ---: |
| Direct Horizons Q46 | 0.01″/h | 243/610 | 0.186528″/h (Moon, year 100) | 0.0794494″/h (Moon, year 1800) |
| Centered difference of Horizons Q45 positions | 0.005″/h | 0/610 | 0.00166374″/h (Moon, year 500) | 0.000642472″/h (Moon, year 500) |

The Q45 finite difference uses ±0.001 TDB day and printed positions. At the same sample epochs, direct Q46 and the Q45 difference themselves disagree by more than 0.01″/h in **244/610** rows; their maximum component differences are 0.186759″/h in RA·cos(dec) and 0.0794503″/h in declination. This is evidence that the comparison or reference construction needs investigation, not proof that either JPL quantity or the engine is wrong. The finite difference has truncation, timestamp conversion and output-rounding effects; the outputs and model conventions are not an independent uncertainty bound. The discrepancy cannot be resolved by silently widening a threshold. Q45 differences favor the sampled refined ICRF velocities only within this diagnostic; they do not validate the ordinary ecliptic six-field speed pipeline, radial rates, extreme epochs, or physical accuracy. Near-solar conditioning and the deflection clamp remain separate concerns.

## Reproduction and evidence

The locally supplied classic DE441 is read-only, SHA-256 `476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da`. The rate tools check the hash before and after and refuse to overwrite output. With the existing Node addon built and network access, from the repository root:

```text
node bindings/node/scripts/compare-horizons-icrf-rates.cjs <kernel> <sha256> <new-q46-report.json> 0-3000-step-50
node bindings/node/scripts/compare-horizons-rates-by-position.cjs <kernel> <sha256> <new-q45-report.json> 0-3000-step-50
```

The Q46 command exits nonzero on the 243 threshold failures. The raw JPL responses and the locally saved `check_lunar_grid_horizons.py` orchestration are kept in the owner's `science-evidence-20261118` directory; the script uses local executable and ERFA paths that require adaptation on another machine. The kernel and JPL responses are not committed here. SHA-256 of the local files from this run:

- `lunar-grid-horizons.json`: `061c5eff8842b2567b34d195f55226cd814193771d6c2a9c6c8e49e189eb1c52`.
- `horizons-icrf-rates-followup.json`: `f216af0431fdbb0a74a23884f4dca770b22d35e32c175531126702be99248b3c`.
- `horizons-position-rates-followup.json`: `ee2db9a381767c57efddaab6048b25b66fb580775c40724c3bd500e3afa138a1`.
- Versioned lunar CSV: `403d64c1c50722da6aa43f8190c780d49cc118573591379fe68d328b1b2e1581` (LF checkout). Historical CSV SHA-256 on this Windows checkout: `ad3c662a424ec5aa7be14f07be07d918c0a3941ccedf0588e17277dd2211cd87` (line endings can change checkout hashes).

Reproduce the snapshot by building `cargo build --release --offline -p virgostars-ephemeris-core --example apparent_six_fields`, running the example as `<exe> <kernel> <tdb_jd> Moon` for each of the 199 historical lunar rows, and copying output fields target, JD and the six numeric fields into a new CSV. Do not overwrite the historical fixture.

Further work must identify the Q46/Q45 difference, check speeds and radial rates against independently defined references, and expand date/target/near-solar coverage. A passing versioned regression is not approval to enable the apparent engine in Mobile.
