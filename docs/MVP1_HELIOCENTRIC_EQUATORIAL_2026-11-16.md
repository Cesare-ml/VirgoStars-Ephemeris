# MVP 2 (early) — heliocentric and equatorial output (2026-11-16)

**Status: Locally tested against the installed `swisseph` 0.5.17. Not integrated.**
These are the `SEFLG_HELCTR` and `SEFLG_EQUATORIAL` modes that the backend calls for
every body (`swe_calc_ut` with helio and with equatorial flags). They belong to MVP 2 of
the plan; they were taken early because the apparent pipeline already had everything
they need.

## What was added

- **Equatorial frame.** `ApparentFrame::TrueEquatorOfDate` gives right ascension
  and declination in the true equator and equinox of date: the same IAU 2006
  precession and IAU 2000B nutation, without the final rotation by the true
  obliquity. The rates include the frame's own motion (`dM/dt`, finite difference of
  the declared matrix), as for the ecliptic.
- **Heliocentric center.** `ApparentCenter::Heliocentric` places the observer at
  the Sun's center: one-way light-time (the photon leaves the body and arrives at
  the Sun at the observation epoch), no annual aberration and no solar deflection,
  which are not defined for that observer. The velocity is the exact derivative with
  respect to the observation epoch. `light_time::heliocentric_light_time` is a new
  public function; `geocentric_light_time` is unchanged. The Earth is a valid target
  here (and only here); the Sun seen from the Sun has no direction and the
  Node binding returns the zero vector for it, as the reference does.
- **Options and results.** `ApparentOptions { solar_light_deflection, frame, center }`
  (default: geocentric, ecliptic, deflection on), results carry `frame`, and
  `applied_corrections`/`not_applied_corrections` describe each mode. The example
  `apparent_six_fields` takes `--heliocentric` and `--equatorial`.
- **Node binding and facade.** `SEFLG_HELCTR` (8) and `SEFLG_EQUATORIAL` (2048) are
  accepted, alone or together, for the bodies 0–9 and, with `SEFLG_HELCTR`, the Earth
  (14). The facade returns `rectAscension, declination, distance, rectAscensionSpeed,
  declinationSpeed, distanceSpeed, rflag` for equatorial requests, like the
  reference. `rflag` is `1|8|256|512|1024` (= 1801) for heliocentric, `1|256|2048`
  (= 2305) for geocentric equatorial, matching the reference's reports. Fixed stars
  still refuse both flags with `E_UNSUPPORTED_FLAG`.

## Evidence

- Core: 2 synthetic tests (defaults and correction lists; the equatorial result is
  the ecliptic one rotated by the true obliquity to 1e-12) and 3 real-kernel tests:
  heliocentric positions equal an independent re-derivation from the raw DE441 states
  for Venus, Mars, Jupiter, Earth and the Moon at four epochs (< 1e-7°); heliocentric
  ecliptic, heliocentric equatorial and geocentric equatorial rates equal exact-step
  centered differences over 1900–2100; the Sun-from-the-Sun and the geocentric Earth
  are rejected, the heliocentric Earth is accepted. Core suite: all pass.
- Node: 38 tests, all pass with kernel, catalog, USNO files and the real
  `swisseph` package (11 pass, 27 skip without data).
- Against the reference, 41 TDB dates 1900–2100 (receipt
  `docs/evidence/mvp1-heliocentric-equatorial-local-reference-receipt-2026-11-16.json`):

  | Output | Maximum difference |
  |---|---|
  | geocentric equatorial, all ten bodies | 0.00066″ in RA and 0.0006″ in Dec |
  | heliocentric, Moon and Earth | 0.0006″ |
  | heliocentric, outer planets | Jupiter 0.016″, Saturn 0.006″, Uranus 0.0025″, Neptune 0.0013″, Pluto 0.0012″ |
  | heliocentric, inner planets | Mars 0.08″, Venus 0.28″, Mercury 0.92″ |

## The heliocentric difference for the planets

The geometric (no light-time) heliocentric positions agree with the reference's
`SEFLG_TRUEPOS` result to 1e-8° (checked for Mars at J2000), so the frame is right.
The difference appears only with light-time and grows toward the Sun. At J2000 the
reference's Mars value equals a variant in which the light-time delay is taken from
the planet's distance to the solar-system **barycenter**, not to the Sun
(359.43841263° against 359.43841267°), while for the Earth it equals the Sun-based
definition used here. That is consistent with a convention of the reference; it is
not proven in general. This engine implements the physical definition: the photon
arrives at the Sun. If bit-for-bit parity with the reference for the inner planets
(up to 0.9″ for Mercury) is wanted, it can be added as a separate, explicit
compatibility profile.

## Not done

- `SEFLG_NOABERR`, `SEFLG_TRUEPOS`, `SEFLG_J2000`, `SEFLG_NONUT`, `SEFLG_BARYCTR`,
  `SEFLG_TOPOCTR` and sidereal modes (rejected with `E_UNSUPPORTED_FLAG`).
- Equatorial or heliocentric output for fixed stars.
- Bodies above Pluto, nodes, apogees and Chiron (the rest of MVP 2).
