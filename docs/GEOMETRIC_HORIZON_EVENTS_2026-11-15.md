# Experimental geometric horizon events

The Rust `horizon_events::find_geometric_event` and Node
`findGeometricEvent` search for a sampled rising or setting center altitude,
or an upper or lower meridian transit. The caller supplies a separately
licensed DE441 kernel, the target, event type, observer latitude and east
longitude, threshold altitude, sampling step and search window. UT1, TT and TDB
Julian days must be supplied explicitly for the same initial instant. Their
offsets are held constant throughout the search; there is no conversion from
UTC, leap-second processing, Earth-orientation interpolation or polar motion.

By default the model projects the geocentric DE441 light-time and annual-aberration
direction onto a geometric horizon using approximate sidereal time. An explicit
`observerHeightM` in Node, or `find_topocentric_geometric_event` in Rust,
subtracts the observer's WGS84 position, producing an approximate topocentric
direction. Height is geodetic metres between -12000 and 100000. The parallax
correction uses the geocentric direction and distance already computed; it does
not recompute light-time from the observer or apply diurnal aberration. Neither
mode applies atmospheric refraction, body semidiameter,
terrain or solar deflection. The specified altitude is ignored for meridian
transits. The target list uses planetary system barycenters where labelled.
This is **not** an implementation of Swiss `swe_rise_trans` or a prediction of
the observed sunrise, sunset, moonrise or moonset.

The window must be greater than zero and at most two days, and the step must
be between one and 60 minutes. The implementation samples for a directional
sign change and bisects the first detected bracket. It may miss tangencies or
multiple crossings between samples. A missing result says only that no crossing
was found in the sampled window. The returned bracket width (at most 0.25 s)
describes numerical termination, not physical accuracy. The latitude must be
strictly between the poles, longitude within ±180°, and altitude threshold
strictly between ±90°.

Example, with `provider` created by `createExperimentalRustCli` and epochs
for the same instant supplied by the caller:

```js
const event = provider.findGeometricEvent({
  target: 'Sun', kind: 'rise',
  ut1JulianDay, ttJulianDay, tdbJulianDay,
  latitudeDeg: 41.9028, longitudeDegEast: 12.4964,
  thresholdAltitudeDeg: 0, windowDays: 1, stepMinutes: 10,
  observerHeightM: 20, // omit for the explicitly geocentric calculation
});
```

Valid kinds are `rise`, `set`, `upper_transit` and `lower_transit`. The Node
result carries an experimental model label and limitation warnings. The
kernel is supplied locally and is not distributed by this repository.

The tests distinguish a synthetic sign-crossing check from a DE441 integration
check: the latter verifies that the computed solar event reaches its own
geometric altitude threshold. A separate fixed diagnostic compares the lunar
direction and center rise/set with Swiss 2.09.03 using the same DE441 kernel,
Rome at 20 m on 2026-11-10, JPL flag and no refraction or limb. UT1=UTC is
assumed for that diagnostic. The recorded rise and set differ by about 0.10 s
and 0.04 s respectively; the 3 s test tolerance covers model and numerical
differences for those two fixtures only. Swiss is a comparison implementation,
not an independent JPL/Horizons validation or proof of general parity.

A broader local diagnostic used the same Swiss version and DE441: 2026 Jan 15,
Apr 15, Jul 15 and Oct 15 at 00:00 UT; sites (latitude, east longitude) (0, 0),
(41.9028, 12.4964), (-33.8688, 151.2093), (51.5072, -0.1276), and (-45, 170),
all at 20 m. For Sun and Moon, it compared center rise and set, without
refraction, over a two-day window with 10-minute samples. The search used
UT1=UTC, TT=UTC+69.184 s and TDB=TT. All 80 events were found; the largest
absolute differences were 0.079 s (Sun) and 0.226 s (Moon). These are observed
differences for this finite grid, not error bounds: high latitudes, grazing
events, other epochs, atmospheric effects and other ephemerides were not
covered. The 0.25 s bisection bracket limits the resolution of this comparison.
