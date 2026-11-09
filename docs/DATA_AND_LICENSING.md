# Data sources, provenance and licensing (2026-11-09)

Status: **verified public starting references; legal and technical review still required for every kernel/dependency actually used**.

## JPL source of astronomical data

- JPL planetary/lunar ephemerides: https://ssd.jpl.nasa.gov/planets/eph_export.html
- Official NAIF planet SPK archive: https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/
- NAIF generic kernels: https://naif.jpl.nasa.gov/naif/data_generic.html
- JPL Horizons independent reference: https://ssd.jpl.nasa.gov/horizons/
- SPK technical specification: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html

**Starting candidates, not an unqualified guarantee of capability:**
- `de440s.bsp`: practical modern-era test kernel (roughly 1849–2150 for relevant segments); inspect per-body segment limits.
- `de440.bsp`: longer DE440 coverage (roughly 1550–2650).
- `de441_part-1.bsp` and `de441_part-2.bsp`: long-span DE441, about 13200 BCE–17191 CE, with a different lunar model. Switching ephemeris families is scientifically meaningful; never silently assume identical values.
- A newer file/solution (including DE442 if relevant) requires separate verification of provenance, applicability, exact coverage and correctness; "newer" does not automatically mean better for the target epoch.

SPK provides ephemeris segments; coverage **varies by target and frame**. Many planetary positions are barycenters, not physical centers. Other SPKs may be needed for minor bodies and specific planet centers. Additional LSK (leap seconds), PCK, frame, and Earth-orientation resources can be required for supported observation modes.

## Rights — distinct assets

1. **Original VirgoStars Ephemeris source**: MIT license applies only to original code and documentation in this repository.
2. **NAIF-supplied SPK kernels**: NAIF states they can be downloaded and used, and that redistribution of kernels NAIF distributed is permitted when they have not been modified; provenance/attribution must be respected. Kernels are **not relabeled MIT** by including a manifest/link.
3. **NAIF SPICE Toolkit**: distinct rules; redistribution of the unmodified standalone toolkit as a mirror requires prior written clearance. Inclusion of toolkit modules within a customer's SPICE-based tool is described separately. **Do not copy/toolkit-vendor without review.**
4. **ERFA or other external code**: licenses are their own; use and preserve all notices, source links, versions and changes; do not claim entire third-party material is ours.
5. **Swiss Ephemeris**: has AGPL/professional dual license. It is a **non-dependency** of this project. Do not paste/copy/translate its implementation or bundle its datasets, compressed ephemerides, source or headers as project artifacts.

Official rules: https://naif.jpl.nasa.gov/naif/rules.html ; Swiss terms: https://www.astro.com/swisseph/swephinfo_e.htm .

## Distribution policy

- Do not commit `.bsp` files or toolkit binaries to Git. Use documented downloads from official NAIF source, optional checksum verification and an **explicit, auditable** cache. Avoid remote fetch on import or calculation.
- Before any file is distributed as part of a package, record source URL, producer, file name, SHA-256, solution version, last verification date, relevant terms, transformation (if any), segment coverage, credits.
- If modifying a kernel, follow NAIF rules requiring corrected attribution and filename; prefer **not modifying** NASA/JPL kernels.
- Record new accuracy/citation references and scientific model in the PR.
- Avoid NASA/JPL/NAIF logos and endorsement claims. Cite the appropriate JPL ephemeris paper in publications; example: Park et al. 2021 for DE440/DE441 (https://ssd.jpl.nasa.gov/doc/de440_de441.html).

## License checklist before public release of code

- [ ] Every dependency reviewed for licensing and notices
- [ ] No Swiss/copyleft or non-redistributable implementation copied into MIT code
- [ ] Exact kernel versions/coverage checked
- [ ] Public documentation contains no secrets/private data
- [ ] Independent scientific test/reference provenance documented
- [ ] Human legal review completed if commercial redistribution/licensing remains uncertain

This document records technical intentions, not legal advice.


## 77-term IAU 2000B scientific table added (2026-11-09)

`src/data/iau2000b-terms.mjs` includes the published numerical 77-term nutation coefficients derived from the BSD-style ERFA `nut00b.c` reference, explicitly credited to NumFOCUS Foundation; see repository `THIRD_PARTY_NOTICES.md` and the full `third_party/ERFA_LICENSE.txt`. These numerical series data are third-party provenance even though the new VirgoStars `src/nutation.mjs` JavaScript evaluator is original source.

Preserve full ERFA attribution and required BSD notices in both source and any binary/package distributions; the general MIT license does not automatically supersede them. No ERFA C source or runtime dependency is bundled and no Swiss code or compressed Swiss data has been copied.
