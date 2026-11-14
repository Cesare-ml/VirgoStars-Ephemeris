# Experimental true ecliptic positions and rates

The Rust core composes the classic DE441 reader with Earth-centered reception
light-time, annual aberration, IAU 2006 precession and IAU 2000B nutation.
The output is in the moving true ecliptic of date; it provides longitude and
latitude in degrees, distance in AU, and all three derivatives per TDB day.
The moving-frame derivative uses a centered 0.01-day matrix difference,
assuming dTT/dTDB = 1. Planet identifiers retain the actual DE441 barycenter
where a physical planet center is unavailable. No external data is bundled.

An offline example reads the file locally and prints ten bodies as CSV:

```powershell
cargo run --release --example true_ecliptic_at -- C:/data/de441.eph 2026-11-10T00:00:00Z
```

The CSV is printed only after all ten calculations succeed. The Rust API
`true_ecliptic::geocentric_true_ecliptic_from_civil` accepts a
`first_sky::CivilTimestamp` with explicit UTC offset. The supported civil
interval is 1972-01-01 through 2027-06-27 UTC, as bounded by the embedded
leap-second table; leap-second labels are rejected. The TDB−TT conversion is
a truncated approximation, and single-f64 JDs limit subsecond resolution.
The lower-level `geocentric_true_ecliptic` takes separate, coherent TT and TDB
JDs. The one-second agreement guard detects gross errors but does not establish
that the epochs are genuinely equivalent. Frame derivatives close to the
1900/2100 model limits fail when either numerical sample leaves the interval.

For Node, build the example with `cargo build --release --example
true_ecliptic_at`, then explicitly supply the resulting binary and local
kernel to the thin process adapter:

```js
import { createExperimentalRustCli } from 'virgostars-ephemeris/experimental-rust-cli';
const ephemeris = createExperimentalRustCli({
  executablePath: 'C:/path/to/true_ecliptic_at.exe',
  kernelPath: 'C:/data/de441.eph',
});
const sky = ephemeris.calculate('2026-11-10T00:00:00Z');
console.log(sky.results[0].longitudeDeg);
const jd = ephemeris.julianDay({year: 2000, month: 1, day: 1,
  decimalHour: 12, calendar: 'gregorian', timeScale: 'ut1'});
const date = ephemeris.reverseJulianDay({julianDay: jd,
  calendar: 'gregorian', timeScale: 'ut1'});
const lunar = ephemeris.calculateLunarPoints('2026-11-10T00:00:00Z');
const equatorial = ephemeris.calculateEquatorial('2026-11-10T00:00:00Z');
const heliocentric = ephemeris.calculateHeliocentric('2026-11-10T00:00:00Z');
```

The synchronous adapter starts one process per ten-body batch and is intended
for isolated experimentation. It validates the CSV contract and returns
immutable, explicitly labelled results; it does not reimplement astronomy.
`calculateEquatorial` provides right ascension and declination in degrees,
distance in AU and corresponding rates per TDB day, in the IAU 2006/2000B
true equator of date. Its light-time, aberration, time-scale and missing
deflection/body-center limitations match the ecliptic result. The date-frame
rotation and its centered derivative are computed by the same Rust routine;
the output is experimental and is not established Swiss parity.
The Julian-day methods are strict calendar arithmetic on the selected
proleptic calendar and time-scale label. They do not convert UTC to UT1, nor
normalize invalid dates or hours outside [0, 24). They are not ABI-compatible
replacements for `swe_julday` and `swe_revjul`.

`ephemeris.calculateHouseAngles({ut1JulianDay, ttJulianDay, latitudeDeg,
longitudeDegEast})` separately returns ARMC, Ascendant, Midheaven and true
obliquity, in degrees. Both dates must refer to the same instant and UT1 must
be supplied explicitly; UTC is not silently substituted. Sidereal time uses
an approximate equation of the equinoxes without complementary terms and does
not include polar motion. Geographic poles and singular intersections fail.
The equivalent Rust functions are `house_angles::from_epochs` and
`house_angles::from_armc`; they supply geometry, not house-system cusps.

This is **not** a drop-in `swe_calc_ut`: there is no solar gravitational
deflection, complete nutation, full ephemeris of physical planet centers,
Chiron, topocentric options,
legacy flags or native Node binding. The modeled derivatives of the corrected
position are not assumed identical to Swiss output speeds. A UT Julian date
alone cannot be interpreted as UTC, TT or TDB without an explicit time model.

## Validation and provenance

The offline regression suite covers analytic/synthetic fixtures and an ERFA
nutation/matrix conformance sample. An optional local-file test checks the
moving-frame derivatives against centered position differences for six bodies.
The DE441 light-time stage has four single-epoch reference vectors recorded
from Horizons; see [the light-time documentation](GEOCENTRIC_LIGHT_TIME_2026-11-10.md).
These do not establish independent accuracy for the full composed pipeline or
Swiss numerical equivalence. The existing CPAL notices and BSD-3 ERFA
coefficient attribution in `THIRD_PARTY_NOTICES.md` apply.

An exploratory local comparison on 2026-11-14 used the installed Node
`swisseph` 0.5.17 with the same local DE441 file, `swe_calc` at the same TT
epoch, `SEFLG_JPLEPH | SEFLG_SPEED` (returned flags 257), and the ten targets
in the CLI. At 1980-01-01, 2000-01-01 12:00, and 2026-11-10 UTC, maximum
absolute longitude differences across the ten targets were respectively
0.00000666°, 0.00000556°, and 0.00000146°. Maximum absolute radial-speed
differences were 0.00005195, 0.00005056, and 0.00003570 AU/day. This small
comparison is diagnostic only: target-center models and correction sets differ,
the reference is not independent JPL validation, and the speed discrepancies
mean that Swiss equivalence must not be claimed. `swe_calc_ut` additionally
uses a different UT-to-TT policy; supplying UTC as UT1 can change results.

The separate [experimental lunar-point model](LUNAR_NODES_AND_APOGEES_2026-11-10.md)
has its own assumptions, reference checks and limitations.

2026-11-14 equatorial follow-up: independent synthetic moving-frame rate
checks and a local same-TT Swiss/DE441 comparison for six bodies at one epoch
pass; these comparisons do not establish independent JPL validation or a
general accuracy bound.

`calculateHeliocentric` provides Sun-centered, light-time-corrected ecliptic
coordinates and rates; annual aberration and solar light deflection are not
applied. The Sun relative to itself has no defined direction: the Node/CLI
adapter reports an explicit all-zero sentinel with `directionDefined: false`;
the Rust spherical API returns an error for the Sun target. Other bodies keep
their DE441 planet-system-barycenter identities. This is experimental, not a
general substitute for Swiss heliocentric flags.

A local diagnostic at matched TT compared the nine non-Sun outputs with Swiss
0.5.17 and the same DE441 at 28 quarterly dates across 1975–2026. Maximum
absolute differences were 0.0001420° longitude, 0.0000122° latitude,
0.0001572°/day longitude rate, 0.0000196°/day latitude rate and 1.88e-7
AU/day radial rate. These samples do not establish a global accuracy bound or
independent validation of the underlying JPL data and physical models.

A diagnostic same-TT comparison with local Swiss 0.5.17 and DE441 covered ten
bodies at 28 quarterly dates across 1975–2026. Maximum absolute differences
were 0.0000615° in right ascension, 0.0000838° in declination, 0.0000740°/day
in right-ascension speed, and 0.0000290°/day in declination speed. The largest
radial-speed difference was 0.0000605 AU/day. These are sample maxima, not
global tolerances or proof that the speed definitions and models agree.

The sweep exposed a light-time solver two-cycle at 1980-10-01 caused by the
single-f64 Julian-date resolution. The solver now accepts only a cycle between
adjacent representable epochs and chooses the candidate with the smaller
light-time residual. Synthetic tests retain failure for nonadjacent cycles and
near-light-speed nonconvergence. A local real-file regression covers the date.

2026-11-14: reconciled the existing civil/aberration and 2000B development
branches, added the moving-frame projection and CLI, and corrected the two
reserved `SE_ECL_NUT` array fields to zero according to the public Swiss API
description. No Swiss source code or third-party kernel is copied or bundled.
