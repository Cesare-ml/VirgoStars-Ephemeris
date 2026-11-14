# Lunar-point branch integration audit — 2026-11-14

The existing `feat/lunar-nodes-apogees-20261110` branch was evaluated in a
separate local worktree against the installed `swisseph` 0.5.17 and the same
classic DE441 file. It was **not integrated** into this branch. The experiment
used TT JD 2461354.5008007409, `swe_calc` and `SEFLG_JPLEPH | SEFLG_SPEED`
(returned flags 257). No Swiss source or kernel is included.

| Quantity | Existing branch | Swiss reference | Observed issue |
|---|---:|---:|---|
| Mean node distance (AU) | 1.0 | 0.0025695553 | Placeholder is not the reference distance. |
| True node distance (AU) | 0.0026722970 | 0.0026645118 | The branch returns current lunar distance, not the reference node distance. |
| Mean apogee longitude (degrees) | 276.168733 | 276.281946 | Different mean-point models. |
| Mean apogee latitude (degrees) | 0 | −3.906236 | Zero latitude is not the reference mean-apogee output. |

The true-node and osculating-apogee longitudes happen to be close for this
sample, but one epoch is not validation. The implementation evaluates the
DE441 state by treating TT JD as TDB, omits the derivative of the nutation
term from its apparent mean-point speed, and substitutes current lunar
distance for an unbound osculating orbit instead of returning an error. These
contracts require correction, independent references and boundary tests before
exposing them as substitutes. This audit does not claim general Swiss parity.

## Follow-up on the lunar integration branch

The subsequent experimental lunar branch corrects the time-scale separation,
nominal mean-node radius, osculating-node radius and unbound-orbit fallback,
and projects the mean apogee from a fixed-inclination lunar orbit. The mean
model is still approximate; two-epoch compatibility regressions and the
remaining model differences are described in
[LUNAR_NODES_AND_APOGEES_2026-11-10.md](LUNAR_NODES_AND_APOGEES_2026-11-10.md).
The first section records the earlier branch before these corrections.
