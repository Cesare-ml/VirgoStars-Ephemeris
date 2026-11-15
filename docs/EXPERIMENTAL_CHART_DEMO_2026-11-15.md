# Offline experimental chart composition

This runnable example connects the existing experimental Node adapter to the
Rust CLI and to explicitly supplied Astrological Functions house providers. It
calculates one instant: a Julian calendar round trip, geocentric planetary
positions and rates, one Placidus house set and planet house positions,
geometric horizontal directions, and caller-supplied fixed-star positions and
magnitudes. Output is JSON with per-result model warnings. It is a smoke test,
not a production chart, Swiss replacement, independently validated catalog or
package release.

Build the experimental binary with
`cargo build --locked --example true_ecliptic_at` and provide separately
obtained classic DE441 and the relevant physical-center SPKs. Provide absolute
paths to the `calculateHouseSet` and `calculatePlacidusHousePosition` modules
from compatible Astrological Functions worktrees, and a caller-supplied star
catalog with documented rights and astrometry. The five-system house provider
and Placidus-position provider have been merged in Astrological Functions PRs
[#118](https://github.com/Cesare-ml/astrological-functions/pull/118) and
[#120](https://github.com/Cesare-ml/astrological-functions/pull/120). The
example still requires explicit paths; it does not install either provider.
No ephemeris, catalog, provider or network access is bundled by the example.

Save an absolute JSON configuration file, replacing all `/absolute/...`
placeholders with actual paths for your platform:

```json
{
  "executablePath": "/absolute/true_ecliptic_at",
  "kernelPath": "/absolute/de441.eph",
  "physicalCenterKernels": {"Mars": "/absolute/mars.bsp"},
  "houseSetModulePath": "/absolute/house-set.mjs",
  "placidusPositionModulePath": "/absolute/placidus-position.mjs",
  "ut1JulianDay": 2461354.5,
  "ttMinusUt1Seconds": 69.184,
  "tdbMinusTtSeconds": -0.0014,
  "datetimeUtc": "2026-11-10T00:00:00Z",
  "latitudeDeg": 41.9028,
  "longitudeDegEast": 12.4964,
  "houseSystem": "P",
  "bodies": [0, 1, 4],
  "planetSolarDeflection": "POINT_MASS",
  "starSolarDeflection": "POINT_MASS",
  "starNames": ["Synthetic example"],
  "provenance": {
    "engineVersion": "caller-engine-version",
    "kernelSetId": "caller-kernel-set",
    "timeModel": "TEST_UT1_EQUALS_UTC_APPROXIMATION",
    "corrections": "CALLER_ASSERTED"
  },
  "catalog": {
    "id": "synthetic-example-not-a-real-catalog",
    "records": [{
      "name": "Synthetic example", "aliases": [], "epochTdbJulianDay": 2451545,
      "rightAscensionDeg": 20, "declinationDeg": -30,
      "properMotionRaCosDecMasPerYear": 120, "properMotionDecMasPerYear": -40,
      "parallaxMas": 50, "radialVelocityKmPerSecond": 20, "visualMagnitude": 1.23
    }]
  }
}
```

Run `node examples/experimental-chart.mjs /absolute/config.json`. Planet IDs
are 0–9, with explicit matching SPKs for IDs 4–9. `starNames` must match the
supplied catalog. The star shown above is fictional. `provenance` identifiers
are caller assertions, not verified hashes or validation. The example writes
no files and does not print the executable or kernel paths.

The example converts **only the specified instant**, using the supplied
TT−UT1 and TDB−TT offsets. The example values are test assumptions and must
not be treated as an authoritative UTC/UT1 conversion or historical ΔT model.
The house bridge checks a loose proximity between supplied UTC and UT1 but
does not calculate DUT1. The user must provide coherent time scales, kernel
coverage and any real catalog provenance. Invalid values, absent kernels,
solar occultations, unsupported polar/circumpolar houses and undefined
azimuths fail instead of falling back. Each calculation launches a process;
large catalogs or repeated calls may be slow. Solar deflection remains a
point-mass approximation and the current star and planet models have documented
limitations in the adjacent fixed-star and finite-solar-deflection notes.

On 2026-11-15 the example and house bridge integration tests also passed using
the Astrological Functions `main` snapshot `07a4142`, an external local DE441,
and an external Mars SPK. This is a reproducible compatibility check for that
snapshot, not a claim of full production integration or scientific validation.
