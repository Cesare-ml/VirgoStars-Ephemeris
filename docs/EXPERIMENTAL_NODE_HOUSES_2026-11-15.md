# Experimental Node house bridge

The optional `houseSupport` configuration of `createExperimentalSwissSubset`
composes the Rust angle provider with injected `calculateHouseSet` and, when
needed, `calculatePlacidusHousePosition` functions from Astrological Functions.
There is no runtime import between repositories, no Swiss dependency and no
implicit fallback. The caller supplies both functions and source identifiers.

```js
const swe = createExperimentalSwissSubset({
  executablePath, kernelPath, convertUt1ToTtTdb,
  houseSupport: {
    calculateHouseSet, calculatePlacidusHousePosition, convertUt1ToUtc,
    engineVersion:'example', kernelSetId:'example', timeModel:'explicit',
    positionContext:{datetimeUtc:'2026-11-10T00:00:00Z',
      corrections:'LIGHT_TIME_ANNUAL_ABERRATION', observerLongitudeDegEast:12.4964},
  },
});
const houses=swe.swe_houses(ut1JulianDay,41.9028,12.4964,'P');
const point=swe.swe_houses_pos(houses.armc,41.9028,houses.trueObliquity,'P',227.5,-2);
```

`swe_houses` accepts only codes V, P, K, R and O. It requires caller-supplied
UT1→TT/TDB and UT1→UTC conversions for the same instant; a two-second guard
checks UTC consistency but performs no conversion. UTC must be explicit and
1972 or later. It returns twelve cusps, ascendant, MC, ARMC and an extra
`trueObliquity` field, with model warnings. Vertex and the other Swiss
auxiliary angles are **not** returned. The injected house-set function
calculates all five systems even when one is requested; if any system fails,
the entire call fails. Polar and singular cases have no fallback.

The default result and cusp array are frozen. For applications that mutate
the cusps, explicitly set `legacyMutableResults: true` on the factory. In
that mode `house`, `ascendant`, `mc` and `armc` are enumerable and mutable;
`trueObliquity`, `model` and `warnings` are non-enumerable and read-only so
legacy numeric enumeration does not treat them as angles. Copy metadata
explicitly when serializing, because JSON omits non-enumerable fields.

`swe_houses_pos` accepts only Placidus `P`, using both ecliptic longitude and
latitude. Because the legacy-shaped signature supplies no instant or source,
the caller must explicitly configure a fixed `positionContext` with UTC,
corrections and observer longitude for that chart. It is the caller's
responsibility to ensure ARMC, obliquity and point coordinates correspond to
that context. Separate charts may need separate instances. Circumpolar points
fail, and no southern inversion or boundary bias is applied. This is not
legacy behavior at all edge cases. Callbacks, if supplied, execute synchronously
and receive the same result returned on success; errors throw.

The automated optional integration test uses the compiled Rust CLI, the two
explicit Astrological Functions module paths and locally supplied DE441. It
checks the five systems and one Placidus point against recorded Swiss 2.09.03
values for Rome, and checks callback and error behavior. This is a limited
compatibility diagnostic, not a scientific validation or production switch.
Set `VIRGOSTARS_HOUSE_SET_MODULE`, `VIRGOSTARS_PLACIDUS_POSITION_MODULE`,
`VIRGOSTARS_EPHEMERIS_CLI` and `DE441_PATH` to run it; no data is downloaded.
