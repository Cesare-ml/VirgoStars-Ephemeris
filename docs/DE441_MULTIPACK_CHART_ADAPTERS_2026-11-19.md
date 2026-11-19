# DE441 multipack chart adapters — 2026-11-19

Status: **experimental core APIs; not activated in Desktop**. Builds on
[#651](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/651).
The existing lunar and stellar formulas now accept a state provider that
selects the correct package independently at every finite-difference and
retarded epoch. Missing packages remain typed errors. Returned state target,
center, epoch and finite values are checked. Existing single-file APIs retain
their signatures. The public lunar and stellar error enums gain pack and
invalid-provider-state variants; downstream exhaustive matches must be
updated before adopting the new revision.

An opt-in small-body apparent API reuses the existing SPK, light-time and
frame pipeline with provider-backed DE441 Earth/Sun states. An opt-in
historical snapshot combines provider-backed planetary candidates, lunar
points and historical house angles at an explicit civil-clock convention and
ΔT. That snapshot deliberately does not provide solar-event searches.
No formula or second astronomical engine was introduced.

Local Windows tests compared these candidate paths with the same full DE441
source at Desktop boundaries and at a synthetic modern boundary: lunar
points in both frames, a fictional stellar catalog entry with and without
solar deflection, a separately supplied Chiron SPK in both frames and
geocentric/heliocentric modes, and complete historical snapshots for years
1000, 2000 and 2201. Missing-neighbor tests returned typed errors. These
comparisons do not constitute independent scientific validation, historical
time accuracy or permission to redistribute any data.

Desktop integration, solar searches, actual UI, signed/authenticated catalog
provisioning, full-file migration, installer and update lifecycle remain
separate work. Long-term planetary/lunar models remain explicitly experimental.
