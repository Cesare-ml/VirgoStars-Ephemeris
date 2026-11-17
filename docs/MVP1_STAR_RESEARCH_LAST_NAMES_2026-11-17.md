# Final unrecorded name hypotheses — 2026-11-17

**Status: caller-asserted research, REVIEW_REQUIRED; not a catalog.** The
[previous review queue](MVP1_STAR_DESIGNATION_REVIEW_2026-11-17.md) had three
names without machine-recorded hypotheses. The new queue retains all prior
candidates and adds research leads for those three; it does not decide identity,
component, quality, magnitude, radial velocity, aliases or rights for any name.

## Research leads

| Legacy input | Unapproved hypotheses | Evidence and limits |
|---|---|---|
| Foramen | η Carinae system | [Astrodienst](https://www.astro.com/astrowiki/en/Foramen) associates this name with η Carinae. [NASA](https://science.nasa.gov/3d-resources/eta-carinae-homunculus-nebula-low-mass-loss-rate/) describes a two-star system; the intended component or system position and variable brightness are undecided. No HIP, coordinates or magnitude are assigned to this lead. |
| Han | ζ Ophiuchi, HIP 81377 | [RASC](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf) associates Hán with ζ Ophiuchi; [SIMBAD](https://simbad.cds.unistra.fr/simbad/sim-id?Ident=Zeta+Ophiuchi) lists HIP 81377. The local HIP2 numerical screen passes both existing filters. Product meaning and the data policy remain open. |
| Wei | ε Scorpii, HIP 82396; δ Herculis, HIP 84379; or the Scorpius lunar mansion | [Astrodienst](https://www.astro.com/astrowiki/en/Fixed_Star_List) lists Wei for ε Scorpii. [RASC](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf) records the historical name Wèi for δ Herculis and a distinct lunar mansion Wěi in Scorpius. Different Chinese characters and tones are collapsed in the legacy spelling. Both proposed HIP2 records pass the existing numerical filters; no meaning is selected. |

These source associations are caller assertions; passing a HIP2 screen does
not establish that the caller's designation-to-HIP claim, the selected
component, data quality, photometry, or rights have been independently verified.
The proposed δ Herculis interpretation represents another historical meaning,
not evidence that the backend intended it. The list of hypotheses is not
guaranteed exhaustive.

## Tooling and data

`attach_star_designation_leads.mjs` adds the Han and Wei stellar hypotheses to
the existing mixed queue and repeats the local HIP2 screen. Its designation/HIP
association remains caller-asserted, and URL syntax checking is not source
authentication. `attach_star_object_leads.mjs` now also accepts the resulting
schema `/4` queue and appends schema `/5` object hypotheses without replacing
the earlier objects or stellar candidates. For `STELLAR_SYSTEM` it records
`COMPONENT_REVIEW_REQUIRED`, rather than declaring every single-HIP path
impossible. For `ASTERISM` it records `NOT_APPLICABLE`; position and magnitude
policies remain `UNDECIDED` in both cases. This is review metadata only: no
system, asterism, centroid or other calculation is implemented.

The previous schema `/2` to `/3` command and output are retained unchanged.
Both tools verify queue/source hashes, reject decided rows and inconsistent
counts, and refuse output overwrite. Source files are caller-supplied; no
external data or coordinates are bundled in this repository.

```powershell
node scripts/attach_star_designation_leads.mjs <mixed-queue.json> <designation-leads-v2.json> `
  <hip2.dat> <new-designation-queue.json>
node scripts/attach_star_object_leads.mjs <new-designation-queue.json> <object-leads-v2.json> <new-final-queue.json>
```

Local artifacts under `VirgoStarsData/star-catalog-review/2026-11-17`:

```text
designation-leads-v2.json              0b2218995bfd2df2aa88cacd6af8e417031cc188a137c5cf8b4028957a1ae33f
star-review-queue-designations-v6.json 7ba35c966ad2772ac55bd723ad874d44931c6b5ac7230ebc455f24726f4fc5dc
object-leads-v2.json                    d63f004d028d2fb6dac8506a5039581d1f69e462f48cf1d5443a0d9389700af3
star-review-queue-complete-v7.json      d2b94603ff19514224a2ea401861fb858526517d871b219cc26a1c5b64728544
```

The filename `complete` means that **at least one recorded lead** exists for
each of the 106 requested names, not that the catalog is complete. The queue
contains 77 original IAU and four original SIMBAD exact-match leads, eight
names with earlier IAU-name research, 12 with designation research, and seven
names with eight object hypotheses. Counts overlap. All 636 decisions remain
null. The remaining work includes identity and component adjudication,
scientific-quality and magnitude/RV policy, data provenance and permissions,
an approved manifest, real backend comparison, and independent validation.
