# Stellar component and designation review — 2026-11-17

**Follow-up:** four approximate cluster-center candidates are described in
[the cluster-center checkpoint](STAR_CLUSTER_CENTERS_2026-11-17.md).

**Experimental local candidates; incomplete catalog.** This follows the
[previous gap review](STAR_CATALOG_GAP_REVIEW_2026-11-17.md). The owner's
choice of correct physical identity, with differences from Swiss signaled,
remains in effect. No production server is switched.

## Six designations added

| Lookup | Local identity | Evidence and caveat |
| --- | --- | --- |
| `,V645 Cen` | Proxima Centauri, HIP 70890 | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=V645+Cen) identifies the same object. The installed Swiss reference returned `star not found`; this is declared on the lookup. |
| `,V2500 Oph` | Barnard's star, HIP 87937 | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=V2500+Oph) identifies the same object. The installed Swiss reference returned `star not found`; this is declared on the lookup. |
| `,al-1Her` | Existing Rasalgethi, HIP 84345 | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=alf+Her) links α Her, Rasalgethi and HIP 84345. The existing catalog uses a five-parameter system solution; it is not a resolved binary orbit. The local Swiss `,al-1Her` direction was nearly opposite to this engine's result, with up to 647,956″ in wrapped longitude at the sampled epochs; the cause is not established and the mismatch is declared. |
| `,pi-4Ori` | π⁴ Orionis, HIP 22549 | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-basic?Ident=pi4+ori) identifies HIP 22549. The local Swiss lookup labels it `Tabit`, while [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=%2A+pi.03+Ori) identifies Tabit as π³ Ori, HIP 22449. The two identities remain distinct and the name difference is declared. π⁴ Ori is a spectroscopic binary; the local linear solution does not model its orbit. |
| `,zeUMa` | Existing Mizar, HIP 65378 | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-id?Ident=MIZAR) identifies ζ UMa with Mizar and HIP 65378. The designation refers to a multiple system; the existing HIP2 entry does not resolve its orbit. |
| `,laCet` | λ Ceti, HIP 13954 | [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-basic?Ident=lam+Cet) identifies λ Cet as HIP 13954. Swiss returns the historical name Menkar, while [SIMBAD](https://simbad.u-strasbg.fr/simbad/sim-basic?Ident=Menkar) associates the current Menkar identity with α Cet, HIP 14135. Both names have historical use: the local lookup selects λ Cet and declares the naming difference. |

The HIP2 filters classify π⁴ Ori and λ Cet as strict for these numerical
criteria; this is not an individual scientific certification. Their local
Hipparcos I/239 V magnitudes are 3.68 and 4.71 respectively; both explicitly
assume zero radial velocity. Existing metadata, including historical radial
velocities and earlier warning claims, are preserved. The source evidence
establishes identifiers, not orbital behavior or all-epoch precision.

## Coverage and verification

The output local manifest `manifest-identity-lambda-simbad-20261117.json` has
**1,071 candidates and 700 aliases**. The exact-string inventory has
**692/729 names and 1,079/1,113 designations**: **37 names and 34 designations
remain**. It contains no missing magnitudes; 1,066 candidates assume zero
radial velocity. These are local lookup counts, not distinct stellar objects,
scientific acceptance or proof of complete Swiss behavior.

The nine-epoch same-DE441 audit attempted **15,939** lookup/epoch combinations
(TT in Swiss, approximately corresponding TDB here). Of these, **15,921** had
both finite results. There were **216** position disagreements above the 1″
diagnostic threshold and **18** unavailable Swiss reference results; all 234
were associated with declared warnings. The latter 18 are the two
Proxima/Barnard designations across all nine epochs. The reference-name
warnings for π⁴ Ori and λ Cet are separate from those 234 failures: their
sampled coordinates agreed within 1″. This comparison neither validates
physical identity nor bounds error between samples.

A real-engine local facade smoke check exercised these six lookups plus Tabit
and Menkar, confirming distinct names and the applicable warnings. A separate
synthetic regression checks that two nearby designations route to different
HIP identities and keep their lookup-specific warnings.

Local SHA-256 receipts:

- Manifest: `155f9632c46f53c3b0929ef8eb52db1b175b2e3a85528663d03faa90359b94ed`.
- Coverage report: `0de9d6b8cd7b9f64f176a852d37eae3fc4b942d307392c3e737fae6b8262c4b6`.
- Century report: `2d4122472dd7c4b37b56c5b8a24ca9f54e8f530f4737d6c052ab523fbe9cd62a`.
- Decision files, in order: `11ecd603310575d9808bcf2cb6cd198888d91de1ab8f8121d0410b8fb902fb4c`,
  `79c6016cd7896678e40d4a78156736da365d5750ba69428125a4944d203890ab`,
  `726fd65a2cf683ed40eb3d2ea1845f9f7deace3cd3bf93446405cc2db03a3315`,
  `6bd2c43a6bf78edbb5f37f0a426b559c4d98699f54ce66845df9d1884e61e4f2`.

Replay the two alias and two HIP decisions in their hash-bound order with the
tools in the [recovery procedure](STAR_CATALOG_RECOVERY_2026-11-17.md), then
run the coverage and nine-epoch diagnostics with new output filenames.
Catalog data and decisions remain local; programs do not authenticate asserted
source claims. The remaining names include shared names such as Kuma and
Jabhat al Akrab, as well as nonstellar sources and reference directions;
selecting one arbitrary component or inventing a parallax would not resolve
their scientific identity.
