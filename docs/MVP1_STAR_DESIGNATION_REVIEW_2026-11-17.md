# Designation hypotheses for stellar review — 2026-11-17

**Status: caller-asserted research; REVIEW_REQUIRED; not an approved catalog.**
The [previous checkpoint](MVP1_EXTENDED_STAR_OBJECTS_2026-11-17.md) left 13
requested names with no machine-recorded lead. A separate supplement now
records 15 designation hypotheses for 10 of those names. It does not choose
between alternatives or establish that the legacy product meant any of them.

| Input | Hypotheses recorded, with candidate HIP | Local HIP2 numerical screen |
|---|---|---|
| Birdun | ε Centauri 66657; Sirius 32349 | traditional only; neither filter |
| Girtab | κ Scorpii 86670; θ Scorpii 86228 | both filters; neither filter |
| Kakkab | α Lupi 71860 | both filters |
| Koo She | δ Velorum 42913 | neither filter |
| Hasseleh | ι Aurigae 23015 | both filters |
| Manus | θ Aurigae 28380; β Tauri 25428 | both filters; both filters |
| Metallach | α Trianguli 8796 | both filters |
| Muhlifain | γ Centauri 61932; α Centauri A 71683; β Centauri 68702 | neither filter for any |
| Peregrini | μ Velorum 52727 | neither filter |
| Rigil | α Centauri A 71683 | neither filter |

The screen checks a numerical selection rule on a local HIP2 record. It does
**not** verify the asserted designation-to-HIP association, the desired
component, the scientific accuracy, source provenance, rights or approval.
The report flags shared HIPs, including Manus/El Nath, Girtab/Sargas,
Muhlifain/Agena/Rigil and Birdun/Sirius; a shared identifier does not establish
that the requested names are interchangeable.

## Identity conflicts requiring review

- [Astrodienst](https://www.astro.com/astrowiki/de/Fixsternliste) associates
  Birdun with ε Centauri. The RASC handbook's
  [2025.6 edition](https://rasc.rocks/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.6.pdf)
  associates Birdun with Sirius, whereas its
  [2025.14 edition](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf)
  separates a Centaurus asterism called Birdun from the Sirius entry. The older
  entry may be an editorial error; Sirius is retained solely to make that
  conflict reviewable, not as a confirmed alias.
- The [RASC handbook](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf)
  identifies Muhlifain with γ Centauri and describes a historically uncertain
  related attribution to α or β Centauri. Similar names occur for other stars;
  the three hypotheses are not claimed to exhaust all historic interpretations.
- [Astrodienst](https://www.astro.com/astrowiki/en/Auriga) associates Manus
  with θ Aurigae; a [historical account](https://penelope.uchicago.edu/Thayer/E/Gazetteer/Topics/astronomy/_Texts/secondary/ALLSTA/Auriga*.html)
  discusses Aurigae Manus in relation to γ Aurigae, now β Tauri. Whether the
  legacy input uses that longer historical designation remains unverified.
- [RASC](https://www.rasc.ca/sites/default/files/World%20Asterisms%20Project%20Handbook%20V%202025.14.pdf)
  associates Girtab with κ Scorpii and discusses an older asterism; a
  [nomenclature discussion](https://groups.google.com/g/stellarium/c/6dd3W0qEtqs/m/K4wR5lInDwAJ)
  notes use for θ Scorpii by some sources. Such use may be mistaken.
- Koo She and Rigil have component/asterism ambiguities. The
  [IAU working-group report](https://www.iau.org/static/science/scientific_bodies/working_groups/280/wg-starnames-triennial-report-2015-2018.pdf)
  describes the attribution of Rigil Kentaurus to α Centauri A; that alone does
  not establish what the shorter legacy input meant. Each claim document carries
  its own further source links.

## Reproducible local artifacts

`scripts/attach_star_designation_leads.mjs` appends caller-provided designation
claims to the mixed queue. It validates the base queue and HIP2 hashes, requires
all decisions to remain null, screens the claimed HIP records, preserves
multiple hypotheses, and recalculates shared HIP flags. It rejects extra claim
fields such as coordinates or magnitudes and refuses to overwrite files. It
validates HTTPS URL syntax only: it does not fetch sources or establish that
the evidence supports the claim. Unlike the earlier IAU-name supplement, this
tool does not check the designation against the IAU snapshot.

```powershell
node scripts/attach_star_designation_leads.mjs <mixed-queue.json> <designation-leads.json> `
  <hip2.dat> <new-designation-queue.json>
```

The artifacts remain outside Git under
`VirgoStarsData/star-catalog-review/2026-11-17`:

```text
designation-leads-v1.json             1aa2403446f0c896b6b22932dd7af2575a785f5df5fb18f7180a4656d424d00a
star-review-queue-designations-v5.json fb287656d8a3edc408178e7053fd7110131dc081e270ae70eb74f69db492cd22
```

The latest queue retains all 106 requests, 77 original exact IAU leads, four
original exact SIMBAD leads, eight earlier stellar research names and five
extended-object names. It adds ten names with designation hypotheses and has
three names without a machine-recorded lead: **Foramen, Han and Wei**. This is
not a count of resolved identities: all six decisions for every entry remain
null. Additional hypotheses may exist even for names with a recorded lead.
Source permissions, accuracy, photometry, radial velocity, object/component
meaning and an approved manifest remain outstanding before backend validation
and release.

The subsequent [last-name research checkpoint](MVP1_STAR_RESEARCH_LAST_NAMES_2026-11-17.md)
adds unapproved hypotheses for the three remaining names. The numbers above
describe this historical queue, not the newer research artifact.
