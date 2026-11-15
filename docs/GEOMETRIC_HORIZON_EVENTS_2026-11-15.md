# Experimental geometric horizon events

The Rust `horizon_events::find_geometric_event` and Node
`findGeometricEvent` search for a sampled rising or setting center altitude,
or an upper or lower meridian transit. The caller supplies a separately
licensed DE441 kernel, the target, event type, observer latitude and east
longitude, threshold altitude, sampling step and search window. UT1, TT and TDB
Julian days must be supplied explicitly for the same initial instant. Their
offsets are held constant throughout the search; there is no conversion from
UTC, leap-second processing, Earth-orientation interpolation or polar motion.

The model projects the geocentric DE441 light-time and annual-aberration
direction onto a geometric horizon using approximate sidereal time. It does
not apply topocentric parallax, atmospheric refraction, body semidiameter,
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
});
```

Valid kinds are `rise`, `set`, `upper_transit` and `lower_transit`. The Node
result carries an experimental model label and limitation warnings. The
kernel is supplied locally and is not distributed by this repository.

The tests distinguish a synthetic sign-crossing check from a DE441 integration
check: the latter verifies that the computed solar event reaches its own
geometric altitude threshold, but is not an independent accuracy validation.
