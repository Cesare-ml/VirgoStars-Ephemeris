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
