# Station direction windows and future UT1 — 2026-11-17

**Status: exploratory sampled comparisons and an illustrative historical
forecast; not an accuracy guarantee.** This extends the [previous JPL station
and historical UT1 analysis](STATION_JPL_AND_HISTORICAL_UT1_2026-11-17.md).
No production API, time policy, or deployment was changed.

## Independent comparison near selected stations

From the previous 112 events, we selected the two station directions nearest
each of four astronomical year seeds: 100, 1000, 1900 and 2500. There are 64
events for Mercury, Venus and the Mars–Pluto system barycenters. The source
searches ±500 days around each 1 July seed, so events can fall in neighboring
years. This is a sample, not a complete inventory.

For each engine station, JPL Horizons observer quantity 31 supplies geocentric,
airless ecliptic-of-date longitudes at points used for fourth-order symmetric
differences. We inspect the direction at both sides of the engine station,
using offsets of 30, 60 and 120 seconds, with a half-step of 0.5 TDB day.
Horizons receives approximately converted TT epochs. Its orientation model
differs from ours, but both calculations use DE441: the orientation and
implementation are independent, the underlying planetary solution is not.

| Distance from engine station | Expected signs at both sides | Indeterminate | Unexpected nonzero signs |
| --- | ---: | ---: | ---: |
| ±30 s | 51 / 64 | 13 | 0 |
| ±60 s | 61 / 64 | 3 | 0 |
| ±120 s | 64 / 64 | 0 | 0 |

At ±30 seconds six Neptune and seven Pluto cases were indeterminate; at ±60
seconds two Neptune and one Pluto cases were indeterminate. The engine itself
had the expected signs at both sides of all 64 selected events at each offset.
The Horizons sign is indeterminate when the estimated speed is within
0.00054 arcseconds/day of zero, assuming nearest rounding of the seven-place
printed longitudes. This allowance covers only printed-angle rounding; it does
not bound numerical differentiation truncation, epoch conversion, frame,
ephemeris or physical error. These signs do not certify a station interval or
an accuracy in seconds. The [JPL Horizons manual](https://ssd.jpl.nasa.gov/horizons/manual.html)
describes its observer quantities and reference conventions.

## A future UT1 scenario at fixed TT

The same matrix supplies 32 selected stations near seeds 2500 and 2950. At
each fixed approximate TT instant, the existing opt-in Espenak–Meeus 2006
polynomial provides a central ΔT = TT − UT1, evaluated at the inferred UT1
decimal Gregorian year. To illustrate sensitivity, independently apply ±σ from
the older Huber extrapolation as presented by [NASA in 2007](https://eclipse.gsfc.nasa.gov/SEhelp/uncertainty2004.html):

`σ = 365.25 N sqrt((N × 0.058 / 3) × (1 + N / 2500)) / 1000` seconds,
where `N = year − 2005`. The year is the inferred UT1 decimal year.

| Seed | Events | Old illustrative σ range | Central model ΔT range |
| --- | ---: | ---: | ---: |
| 2500 | 16 | 612–615 s | 1460–1465 s |
| 2950 | 16 | 1730–1735 s | 4062–4073 s |

At fixed TT, adding ΔT moves UT1 earlier by the same amount, so the illustrative
UT1 offsets are `−ΔT ± σ` seconds from TT. This applies an old uncertainty
scenario around a separately defined central model: it is **not** a calibrated
confidence interval for this model, a modern uncertainty assessment, or a
guarantee. It excludes station and reference-frame errors. UT1 is not UTC or
historical/local civil time, and future leap seconds are not predicted here.
Horizons quantity 30 in the future is not used as if it were TT−UT1.

## Reproduction and provenance

The commands below run from `bindings/node`. Supply a local DE441 file; none is
redistributed. The input reports are the previously generated local artifacts.
The JPL run requires network access; both scripts refuse to overwrite output,
check input hashes before and after, and save their complete observations or
computed rows locally. Reports are intentionally not checked into the repo.

```text
node scripts/compare-horizons-station-windows.cjs <de441> 476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da ../../local-data/planet-validation-0-3000/horizons-ecliptic-stations-precision.json 4b82501d509d27b5575b8a2faa8b4733c5cea03b9113624338c266a11ce2a38d <new-output.json>
node scripts/audit-future-station-ut1.cjs <de441> 476096486def4e41bfceb29aa27f50784da0bce318902bcf7b88caad058cd4da ../../local-data/planet-validation-0-3000/swiss-station-matrix-50y.json f87549dc4662245d2b63333ae8bd08fb37fe31ec1696ee36bd4b49f6bcd50699 <new-output.json>
```

Local result hashes, SHA-256:

- `horizons-station-windows.json`: `0861de3f5f17e479cb57f52ac9399c28e9fc287487c67f0cf6c7e9d9bf647531`.
- `future-station-ut1.json`: `25142f442da3401d2bd0c8b6816188917833668601ff3553ef69a3e681f8edf9`.

The Node tests cover stencil construction, the published rounded NASA examples,
and the sign and units of fixed-TT propagation; online results are not fetched
by the test suite.
