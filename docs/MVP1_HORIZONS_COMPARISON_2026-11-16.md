# MVP 1 gate 1 — comparison with JPL Horizons (2026-11-16)

**Status: Locally tested against an independent reference that is not Swiss.**
This is the Horizons comparison required by the MVP-1 exit gate on scientific
truth. It covers the ten planetary targets of the Node binding
(`Engine.calcTdb`: apparent geocentric, solar deflection on) on 25 epochs between
1900 and 2100. It is not a release and not a production switch.

## Reference and method

JPL Horizons observer tables from the geocenter (`500@399`), airless apparent
quantities, epochs given in TT, on 25 dates (TDB JD 2415100.5 + k·2960.7 d). Horizons
states that it uses DE441 for target and center and applies relativistic light
bending by the Sun. Mars to Pluto are the system barycenters (Horizons commands
4–9), as in the engine. Only public queries were sent (body ids, geocenter,
epochs). No Horizons rows are stored; the aggregates are in
`docs/evidence/mvp1-horizons-comparison-receipt-2026-11-16.json`. The tool is
manual and needs the network:
`node bindings/node/scripts/compare-horizons-apparent.cjs <de441.eph> [out.json]`.

## Result 1 — separations between bodies: independent of the frame

Horizons' RA/Dec are expressed in a frame that differs from ours (next section),
and a frame rotation moves every body the same way. The angular **separation**
between two bodies does not change under any rotation, so it tests light-time,
aberration and deflection on their own. 45 body pairs × 25 epochs:

| Pairs | Max difference |
|---|---|
| without the Moon | **0.000008″** (8 µas) |
| with the Moon | 0.0012″ (about 2 m at lunar distance; cause not identified) |

## Result 2 — absolute RA/Dec: a documented frame difference

Horizons itself states that its apparent RA/Dec refer to an "EOP-corrected
IAU76/80" equator and equinox of date, and that its equinox is offset **−53 mas**
from the frame defined by the IAU 2006/2000A system. The engine uses IAU 2006
precession with IAU 2000B nutation. The absolute differences are accordingly the
same for every body and smooth in time: about +0.23″ in RA in 1900, about +0.05″
between 1962 and 2026, about −0.16″ in 2100 (declination up to 0.09″ at the ends).
Inside Horizons' EOP-data span (1962–2026, 8 of the 25 epochs): the RA difference
is 0.046″–0.053″; after removing the stated 53 mas it is **−7.4 mas … −0.2 mas**,
and the declination differs by at most 1.1 mas. Outside that span the secular
drift is the difference between the two precession-nutation models, not an error
of either implementation.

## Result 3 — range and range rate

- Range (`delta`): relative difference ≤ 3e-11 for all bodies except the Moon
  (1.1e-8, about 4 m).
- Range rate: the engine's value equals the centered difference of the range
  **that Horizons returns** to better than 1 mm/s (Mercury 17.740075 vs 17.740074
  km/s; Mars −13.033240 vs −13.033240; Moon 0.070360 vs 0.070360). Horizons' own
  `deldot` differs from the derivative of its own `delta` by 1.6 m/s (Mercury),
  0.7 m/s (Mars), 6 mm/s (Moon), a definitional difference, so `deldot` is not a
  usable reference for the true range rate. The apparent angular rates are
  verified elsewhere against centered differences of the positions
  (see the apparent-position document), and the positions agree with Horizons as
  above.

## What this establishes and what it does not

- Two independent references now agree with the engine: Swiss (local, 0.0007″ in
  positions) and Horizons (8 µas in separations; frame-model differences understood).
- The 8 µas figure is for 10 targets on 25 epochs; the Moon's 1.2 mas is open.
- Not covered: Horizons' own accuracy limits, topocentric quantities, epochs
  outside 1900–2100, fixed stars and houses (Horizons does not provide them).
- Selecting Horizons' frame as the reference for absolute RA/Dec would require
  implementing IAU76/80 with EOP corrections; that was not done and is not needed
  to establish the geometry above.
