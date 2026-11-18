# MVP: historical local clock and house angles

`historical_civil_time::local_to_ut1` accepts an explicit proleptic Julian or
Gregorian date (astronomical year numbering), a caller-selected offset, and a
caller-estimated uncertainty. The offset is local clock minus UT1, east
positive. `LocalMeanSolar` uses longitude × 240 seconds; `Supplied` requires
a source label and may represent a separately researched historical offset.
This does not discover legal time zones, daylight saving, clock conventions,
calendar reforms, or UT1−UTC. A local apparent solar time is not mean solar
time. Select the convention from the actual record; do not silently assume it.
See [USNO, Equation of Time](https://aa.usno.navy.mil/faq/eqtime).

`mvp_snapshot::experimental_snapshot_from_civil` uses that UT1 instant for the
existing experimental planets, lunar points, house angles and optional solar
events. Supply Delta T and its uncertainty separately; the civil uncertainty
is retained but is not propagated into an angular uncertainty. Long-range
orientation, lunar and planetary qualifications from the existing MVP apply.
The optional Espenak–Meeus model is limited to astronomical years −1999..3000;
outside that range a separately justified Delta T must be supplied.

Each planetary entry also contains an apparent geocentric equatorial direction
(right ascension and declination in degrees, true equator of date). It comes
from the same experimental ecliptic direction and long-term true obliquity as
the solar-event path. Equatorial rates are not supplied. This fills the
coordinate fields needed by Desktop without adding astronomy to the client;
the long-term model is still experimental.

`bindings/node.createExperimentalHistoricalHouseAngleProvider()` is an
independent, opt-in UT1/TT angle provider. It does not load DE441. The separate
`astrological-functions/historical-house-bridge` consumes it and supplies the
Porphyry, Placidus, Regiomontanus and Koch cusps. It does not claim UTC for
ancient timestamps; unsupported polar geometry is reported rather than
replaced. Both repositories need their corresponding feature branches until
merged, and no package or Desktop integration is published by these changes.
