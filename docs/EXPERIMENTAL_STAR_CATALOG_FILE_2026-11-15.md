# Experimental external fixed-star catalog file

`loadExperimentalStarCatalogFile` loads a caller-provided UTF-8 JSON file from
an absolute local path. It verifies an explicit lowercase SHA-256 digest against
the file bytes, requires source and license labels, and applies the same record
and duplicate-name checks as the experimental fixed-star provider. Files must
be nonempty regular files of at most 64 MiB. No network access, download,
catalog data, data license or automatic source selection is included.

```js
import {loadExperimentalStarCatalogFile} from 'virgostars-ephemeris/experimental-star-catalog-file';
import {createExperimentalLegacyBridge} from 'virgostars-ephemeris/experimental-legacy-bridge';

const {catalog,provenance}=loadExperimentalStarCatalogFile({
  filePath:'/absolute/path/to/caller-catalog.json',
  sha256:'<64 lowercase hexadecimal characters from a trusted manifest>',
  source:'<caller-verified source and version>',
  license:'<caller-verified data license and attribution>',
});
const swe=createExperimentalLegacyBridge({
  // Supply the explicit executable, kernel, time, flags and other policies.
  ...options, catalog,
});
// Store provenance with results yourself; it is not automatically attached.
```

The file contains the `id` and `records` object documented in
[EXPERIMENTAL_FIXED_STARS_2026-11-15.md](EXPERIMENTAL_FIXED_STARS_2026-11-15.md).
All records must use its explicit ICRS, epoch, proper-motion, parallax and
barycentric radial-velocity conventions; conversion from other formats is the
caller's responsibility. Missing values are rejected rather than guessed.
The returned catalog, record list, record objects and alias lists are frozen;
the provider also snapshots the fields it uses on construction. Retain the
original bytes and trusted digest if reproducibility matters.

The digest proves only that read bytes match the caller's supplied digest; it
does not authenticate an untrusted digest, validate astrometric accuracy,
prove the source labels, or grant redistribution rights. The metadata are
caller assertions, and are not propagated automatically to individual star
results. A vetted and rights-reviewed real catalog remains an MVP requirement.

The [HYG 4.1 documentation](https://github.com/astronexus/HYG-Database/blob/main/hyg/README.md)
identifies a CC BY-SA 4.0 license and documents J2000 coordinates, distance,
proper motion and radial velocity when known. Its description alone does not
resolve all conventions and missing-value semantics needed by this interface.
No HYG data is included or converted, and no compatibility or legal conclusion
is asserted; a source-specific scientific and rights review remains necessary.

## Optional Hipparcos I/311 importer (stacked experimental PR)

The [Hipparcos 2007 fixed-star import guide](HIPPARCOS_FIXED_STAR_IMPORT.md) describes a fully offline, checksum-pinned conversion of the official CDS `hip2.dat` plus a **separately reviewed** TSV containing explicit Johnson V photometry, barycentric radial velocities and cross-identified names. Its [thirty SIMBAD-linked targets](data/fixed-stars-target-crossmatches.json) are research leads, **not an installable measured star catalog**. The importer's outputs are compatible with this loader's `{id,records}` shape; the separate per-star audit must be retained by the caller. Do not describe this PR as a real catalog release or numerical verification.
