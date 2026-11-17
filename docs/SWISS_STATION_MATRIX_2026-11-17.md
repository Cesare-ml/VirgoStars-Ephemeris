# Sampled Swiss station matrix — 2026-11-17

**Status: exploratory diagnostic, not a release accuracy guarantee.** This
extends [the initial eight station comparisons](SWISS_APPARENT_RATE_DISCREPANCIES_2026-11-17.md).
No engine behavior, event API or production configuration is changed.

The [subsequent numerical-stability investigation](STATION_NUMERICAL_STABILITY_2026-11-17.md)
examines short-window fluctuations and Horizons direction derivatives.

## Scope and method

For each of Mercury, Venus and the Mars–Pluto system barycenters, we searched
within ±500 days of 1 July 12:00 TDB in astronomical years 50–2950 every 50
years. For each seed, we selected the nearest positive-to-negative and the
nearest negative-to-positive crossing of a fourth-order finite derivative of
Swiss's **apparent ecliptic-of-date longitude**. This selects 59 years × eight
targets × two directions = **944 station cases**. It is a sampled matrix, not
a complete list of events or a continuous coverage guarantee.

At each crossing, we bracketed (1) the engine's apparent longitude-rate zero,
(2) the derivative of Swiss positions, and (3) Swiss's published longitude
speed. The Swiss reference used caller-installed Swiss Ephemeris 2.09.03 with
the JPL and speed flags checked on every call. Swiss receives approximate TT
corresponding to engine TDB. Both use the same local DE441, but their reference
frame, precession/nutation and correction implementations may differ. Each
root bracket is narrower than 0.00001 day (0.864 s); this is numerical
resolution, **not** a physical accuracy estimate.

All 944 selected cases were bracketed; 472 enter and 472 leave retrograde
motion. Their apparent solar separations were about 14.8°–146.0°. No kernel or
Swiss binary is committed. The source kernel's SHA-256 and provenance limits
are given in the previous reports.

## Differences at sampled stations

Times below are absolute differences between numerical bracket centers in
seconds; the percentiles summarize the selected cases only:

| Comparison | Median | 95th percentile | Maximum |
| --- | ---: | ---: | ---: |
| Engine rate vs derivative of Swiss positions | 0 | 0.58 s | 1.73 s |
| Swiss published rate vs derivative of Swiss positions | 2.88 s | 222.06 s | 998.99 s |

For Swiss-position derivatives, repeating the 0.02-day half-step with 0.01
and 0.04 days changed bracket centers by at most 0.58 s in this matrix. This
is a consistency observation for these samples, not proof of sub-second
physical or historical-time accuracy. A TDB event must not be presented as a
known historical civil time without independent time-scale assumptions.

Maximum absolute differences between the Swiss published-speed zero and the
engine zero were approximately: Mercury 1.2 s, Venus 5.2 s, Mars 12.7 s,
Jupiter 12.1 s, Saturn 46.1 s, Uranus 26.0 s, Neptune 80.2 s and Pluto
**999.0 s** (16 min 39 s). The Pluto maximum was an entry into retrograde
motion near the 2100 seed, with solar separation about 96.9°. The large Pluto
effect is specific to this sampled grid; it is not a universal upper bound.

For 441 cases whose two bracket centers differed by more than three seconds,
we evaluated both published Swiss and engine rate signs once, at the midpoint:
440 had opposite signs, while one Uranus case had the same sign. A closer
inspection of that exception found the Swiss published rate varying at very
small magnitudes near zero. Opposite signs at one sampled instant do not prove
that every instant between two numerical roots has opposite signs, especially
in the presence of numerical noise.

## Correction-mode experiment

For the same 944 seeds, we located the corresponding crossing **separately in
each Swiss mode** and compared Swiss's speed zero with the zero of the
derivative of Swiss positions in that same mode. Different modes have different
coordinates/corrections; their station times are not directly interchangeable.
All 4,720 mode/case pairs were bracketed. Absolute differences in seconds:

| Swiss mode | Median | 95th percentile | Maximum |
| --- | ---: | ---: | ---: |
| Default apparent | 2.47 | 222.47 | 997.83 |
| Aberration disabled | 2.47 | 218.35 | 1045.62 |
| Deflection disabled | 2.47 | 222.47 | 998.66 |
| Nutation disabled | 0.82 | 7.42 | 21.42 |
| True position | 1.65 | 219.18 | 1043.98 |

The much smaller differences without nutation link the measured station
discrepancy to the nutation option in this version and grid. This is not proof
of Swiss's internal implementation, and disabling nutation changes the
coordinate system; it is not a suggested fix. The true-position mode changes
multiple corrections. The [Swiss flag documentation](https://www.astro.com/swisseph/swephprg.htm)
describes those options.

Independent bisections of the **same** default Swiss event using different
starting brackets changed some measured Swiss-root differences by up to about
5.6 s, despite narrower final brackets. This and the Uranus midpoint exception
show that bracket width alone understates practical numerical uncertainty in
some cases. Further checks should use higher-precision reference positions and
an agreed release budget before deciding how to present station times.

## Reproduction and local evidence

After building the native binding, run these offline commands from
`bindings/node` with the caller's classic DE441, its SHA-256 and a locally
installed Swiss module. Output paths must be new:

```text
node scripts/compare-swiss-station-matrix.cjs <de441> <sha256> <swisseph> <new-matrix>
node scripts/audit-swiss-station-modes.cjs <de441> <sha256> <swisseph> <matrix> <matrix-sha256> <new-mode-report>
```

The matrix script records missing or unresolved roots and exits nonzero if
any occur; the mode audit does the same and verifies the matrix and kernel
hashes and Swiss version. Local matrix SHA-256:
`f87549dc4662245d2b63333ae8bd08fb37fe31ec1696ee36bd4b49f6bcd50699`.
Local mode report SHA-256:
`450bb814855f3a233e8a3865512729702457664d54980200885fa355398f6a73`.
Hashes bind local files, not the authenticity or scientific accuracy of their
sources. Reports and external inputs remain ignored local data.
