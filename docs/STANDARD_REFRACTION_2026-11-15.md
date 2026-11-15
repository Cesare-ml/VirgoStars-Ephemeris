# Experimental standard-atmosphere refraction

`noaa_standard_refraction` is a Rust implementation of the piecewise solar
position refraction approximation published by [NOAA](https://gml.noaa.gov/grad/solcalc/calcdetails.html).
The Node prototype exposes it as `calculateStandardRefraction`. The caller
supplies geometric altitude in degrees, in the explicitly restricted range
[-1, 90]. The result reports the input, the correction, and their sum as an
approximate refracted altitude; no azimuth is changed.

The model uses a fixed standard atmosphere: **it does not accept or infer
local pressure, temperature, humidity, observer altitude or weather**. It is
an approximation of refraction, not a model of visibility, terrain, limb,
topocentric parallax or observer orientation. It is separate from the existing
geometric `calculateHorizontal` and is never applied silently. At and above
85 degrees the NOAA approximation returns zero correction. Below the horizon,
the modeled direction does not imply that an object is actually visible.
The piecewise approximation has small discontinuities at its interval
boundaries and should not be treated as a smooth physical model.

```js
const horizontal = provider.calculateHorizontal({
  ut1JulianDay, ttJulianDay, latitudeDeg, longitudeDegEast,
  rightAscensionDeg, declinationDeg,
});
const refraction = provider.calculateStandardRefraction({
  trueAltitudeDeg: horizontal.trueAltitudeDeg,
});
```

The input equatorial direction must already contain whichever observer and
physical corrections the caller intends. The model is not a drop-in
replacement for `swe_azalt` or `swe_refrac`; in particular it neither accepts
atmospheric parameters nor follows Swiss behavior below the horizon.

Tests evaluate the NOAA published equations at representative values and
boundaries; these formula checks do not constitute observational validation.
For a comparison only, Swiss Ephemeris 2.09.03 at 1013.25 hPa and 10°C gives
apparent altitudes of 0.484586° at 0° geometric and 1.363564° at 1°;
the present approximation gives 0.481944° and 1.363367° respectively.
Actual near-horizon refraction is variable; neither comparison establishes
an accuracy bound for a particular sky or meteorological condition.

## Conventional solar events

For the separate task of estimating sunrise and sunset, Rust exposes
`find_standard_solar_event` and Node exposes `findStandardSolarEvent`. These
use the [USNO conventional definition](https://aa.usno.navy.mil/faq/RST_defs)
of the geometric solar center 50 arcminutes below a level horizon; the fixed
value combines average atmospheric refraction and mean solar semidiameter.
The method searches the geocentric solar direction and takes explicit UT1,
TT, TDB, latitude, east longitude, sampling step and search window. It does
not use `noaa_standard_refraction` or take local weather and height parameters.
It inherits the event search's two-day limit and its possibility of missing
grazing or multiple crossings. Its returned numerical bracket is not an
estimate of observed accuracy. The result is not Swiss `swe_rise_trans` parity.

For a diagnostic at Rome on 2026-11-10 (UTC assumed equal to UT1), the results
were approximately 16 s later for sunrise and 16 s earlier for sunset than
Swiss 2.09.03 default solar events, using the same DE441. These two comparisons
are not a general error bound; actual weather and obstructions can change
observed events substantially.

A further local diagnostic checked 40 solar events: rise and set on 2026
January 15, April 15, July 15 and October 15 at (latitude, east longitude)
(0, 0), (41.9028, 12.4964), (-33.8688, 151.2093), (51.5072, -0.1276),
and (-45, 170). The search began at 00:00 UTC with UT1=UTC, TT=UTC+69.184 s,
TDB=TT, a 2-day window and 10-minute cadence. Swiss used its default solar
rise/set settings with the same DE441 and sea-level observer. All 40 events
were found; the absolute differences ranged from approximately 9.9 to 22.2 s.
This finite sample does not cover grazing events, polar latitudes or changing
weather, and it does not establish an accuracy limit for other dates or sites.
