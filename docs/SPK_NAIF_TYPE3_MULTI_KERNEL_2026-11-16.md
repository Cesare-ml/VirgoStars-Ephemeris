# Local SPK type-3 and multi-kernel comparison — 2026-11-16

**Status: 78 local comparisons passed; scientific and provenance review still
required.** These are geometric numerical comparisons, not a certification of
the physical accuracy of either ephemeris or a general SPICE compatibility claim.

## Sources and checks

- [NAIF jup120_1996-2010.bsp](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/a_old_versions/jup120_1996-2010.bsp),
  1,837,056 bytes, SHA-256 `a36fc7c926b5db607cdcf22638cf15126735f75f8b154ce43926ce1e7a74e98c`.
  Its MD5 `55b7349068f49e146b3d5edc5f7d331e` matches the
  [NAIF checksum list](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/a_old_versions/aa_checksums.txt).
  This detects accidental differences but is not an independent signature.
  Its [NAIF comment](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/satellites/a_old_versions/jup120_1996-2010.cmt)
  identifies this as a historical 2000 solution and explains that its
  planetary components use DE403. It must not be treated as a current
  planetary ephemeris or silently mixed into a product.
- `de440s.bsp`, 32,726,016 bytes, SHA-256
  `c1c7feeab882263fc493a9d5a5b2ddd71b54826cdf65d8d17a76126b260a49f2`,
  from the [NAIF planets directory](https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/de440s.bsp).
- Local independently installed SpiceyPy 8.4.0 / CSPICE N0067 generated
  `spkezr` references. The repository's Rust comparator ran separately.
  Frame J2000, corrections NONE, TDB seconds past J2000, km and km/s.
  The files were hashed before and after comparison.

The local inspector found `599 -> 5`, frame 1, type **3**, in jup120, with
coverage ET −126273600 to 315576000 seconds; the same file also carries
overlapping type-2 planetary chains. It also contains type-17 and frame-2
segments, **not** covered by this comparison. The check does not claim to
support those segments. NAIF's [SPK specification](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/spk.html)
describes type 3 as independent Chebyshev series for position and velocity.

## Matrix and result

Two load orders were tested: `[de440s, jup120]` and `[jup120, de440s]`.
For each, 35 cases combine target:observer pairs `599:5`, `599:399`,
`399:0`, `301:399`, `5:0` with ETs −126273600, −126273599.99999999,
−1, 0, 100000000, 315575999.99999994 and 315576000. The near-endpoint
values are the adjacent representable f64 values on the inside. Four more
cases use `399:0` and `5:0` one second outside each jup120 coverage endpoint,
where DE440s remains available. Those outside cases do not test Jupiter center
599 outside its coverage.

| Kernel order | Cases | Worst position component | Worst velocity component |
|---|---:|---:|---:|
| DE440s, jup120 | 39/39 | 1.192092896e−7 km | 1.065814104e−14 km/s |
| jup120, DE440s | 39/39 | 2.384185791e−7 km | 7.105427358e−15 km/s |

Both used predeclared diagnostic component limits of `1e-5 km` and `1e-8
km/s`, not reviewed physical error budgets. The CSPICE Earth barycentric X
component at ET 0 differed by about 109.25 km between the two orders, so the
overlap check did exercise a real priority-dependent change. Agreement with
CSPICE does not make mixing those historical and modern solutions scientifically
appropriate. The matrix covers one type-3 segment and these specific chains;
it does not establish all within-file segment overlaps, pathological cycles,
non-J2000 transformations, event pipelines or apparent corrections.

The generated reference SHA-256s are
`3b510709b6f0f11f693d504adb3f0c2684ed4500bb653fe269b03f3cbe11562c`
and `868ea11b41885bf969fa49c144880b78357c330ffe171ef77d33168b6d3dcc82`;
the unsigned receipt SHA-256s are
`e6372204b6393efc1ed602a674fc92d5902090fcc38359d8704783dcf7e22674`
and `8ada74ecdec1061e00894e1d4d0ff7cb2eb7217b4685b8c78380e2255e2c0474`.
The operator retained copies of both kernels, references and receipts, plus
the previous 80-case evidence, in `VirgoStarsData/spk-validation/2026-11-16`
outside Git and outside build output; all 12 copies were SHA-256 checked. The
receipts preserve their original run-time paths, not the copied paths. No
third-party kernel or raw reference row is redistributed in the repository.

## Reproduction

With independently installed SpiceyPy and Cargo dependencies already present,
run from the repository root. The data and output paths must exist and both
output filenames must be new; the tool refuses to overwrite them. No downloads
occur in this command. Substitute the actual local kernel paths.

```powershell
$cases = @()
foreach ($pair in @('599:5','599:399','399:0','301:399','5:0')) {
  foreach ($et in @('-126273600','-126273599.99999999','-1','0','100000000','315575999.99999994','315576000')) {
    $cases += @('--case', "${pair}:$et")
  }
}
foreach ($pair in @('399:0','5:0')) {
  foreach ($et in @('-126273601','315576001')) { $cases += @('--case', "${pair}:$et") }
}
python scripts/spk_naif_reference.py run --kernel <de440s> --kernel <jup120> @cases `
  --reference <new-reference.csv> --receipt <new-receipt.json> `
  --max-position-km 0.00001 --max-velocity-km-s 0.00000001
# Repeat with the kernel order reversed and different output paths.
```

## Cross-body and reverse-direction matrix

Another local run compared **196 real-kernel states**: 98 in each of the two
kernel orders. The 14 directed target:observer pairs are `599:5`, `5:599`,
`599:399`, `399:599`, `599:301`, `301:599`, `599:0`, `0:599`, `399:301`,
`301:399`, `399:5`, `5:399`, `5:301` and `301:5`. They cover the same seven
ETs listed in the first matrix, including the type-3 coverage endpoints and
their adjacent interior representable values. These are J2000 geometric states
with no aberration, ET in TDB seconds past J2000, km and km/s. The same
predeclared per-component diagnostic limits of `1e-5 km` and `1e-8 km/s` were
used; they are not independently approved physical error budgets.

| Kernel order | Cases | Worst position component | Worst velocity component |
|---|---:|---:|---:|
| DE440s, jup120 | 98/98 | 1.192092896e-7 km | 1.421085472e-14 km/s |
| jup120, DE440s | 98/98 | 2.384185791e-7 km | 1.421085472e-14 km/s |

For each order the new `audit-reciprocity` command verified 49 forward/reverse
pairs in the **reference CSV** at identical ETs; the maximum component sum was
zero for this matrix. This separate consistency check is not an independent
observation, a proof of physical correctness, or a substitute for the Rust
comparison. It checks metadata, required reverse rows and caller-supplied
tolerances, and verifies ordered kernel and generator SHA-256 before and after.
The chosen limits for this reference check were zero for both quantities.

The two references, unsigned receipts and byte-identical historical generator
are retained outside Git/build output in
`VirgoStarsData/spk-validation/2026-11-16/cross-body-reverse-20261116`.
Their SHA-256 values, respectively, are:

```text
de440-first.csv       b240bab89464ff455c58b6a05e9a2735eef29f1a13d68d3bc03d65d8eae94a6a
de440-first.json      4b1bd167e0a7fc2202b150b9290541a8cabdafd1685c04deb9ea171e5e8ca568
jup-first.csv         4295c3134fe551614edb29240a7b331297e6e0220406a47632171a23b5cc5631
jup-first.json        224fc83cab38cc792aa35d9b8a51620600ecc4dbb67e15e5125dcbf45cd7b318
generator-c6fff22.py  54f017b20222a9733a7123b8e7e2eb83783c3ce915d0018e6030d38a3b76e80f
```

To reproduce the state comparison, use the `run` command above with these 14
pairs instead of the five original pairs, omit the outside-coverage cases, and
use new output paths for both file orders. To audit an existing output:

```powershell
python scripts/spk_naif_reference.py audit-reciprocity `
  --reference <reference.csv> --kernel <first.bsp> --kernel <second.bsp> `
  --generator-source <original-generator.py> --position-limit-km 0 --velocity-limit-km-s 0
```

`--generator-source` supplies the original generator **bytes** when they differ
from the current script; the file is hashed, never executed. Omit it for a
reference generated by the exact current source. Hashes are not signatures and
do not authenticate the source or the scientific validity of combining the
historical and modern kernels.

## Additional synthetic precedence regression

A second CSPICE-generated synthetic matrix tests **176 additional cases**:
eight ordered target/center pairs, 11 epochs and both file orders. Fictional
objects branch from a common ancestor, include a deeper child and reversed
pairs, and a higher-priority segment changes the selected center during an
overlap. Epochs include both endpoints and adjacent representable values.
The comparison passed against CSPICE with diagnostic component tolerances of
`1e-9 km` and `1e-11 km/s` in J2000, geometric, TDB seconds past J2000.
This adds software evidence for these constructed chains, not real-kernel
coverage or scientific approval. Run the same `unittest` command below.

```powershell
python -m unittest scripts.tests.test_spice_synthetic_priority -v
```

With independently installed SpiceyPy, the optional
`scripts/tests/test_spice_synthetic_priority.py` asks CSPICE to write two
temporary fictional type-3 kernels. One has overlapping segments for the same
target and a second target/center chain; the other overlays that target. It
asserts CSPICE selects the later segment/file, reverses file order, and compares
22 target/epoch cases per order against Rust, including exactly at and one
representable f64 step around segment boundaries. All **44 comparisons passed**
locally with diagnostic bounds `1e-9 km`, `1e-11 km/s`. Independent position
and velocity coefficients are deliberately used. This is a synthetic format
and precedence regression, not real orbit accuracy evidence. Temporary kernels
and generated references are not distributed or retained.

**Modification record:** documented a local independent comparison; no SPICE
code, third-party kernel or reference table was added to the repository.
