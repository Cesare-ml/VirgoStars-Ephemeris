# VirgoStars — Free/Open-Source and Attribution Policy (proposed)

**Status: project-wide OWNER PREFERENCE RECORDED; LICENSE CHOICE NOT YET FINAL.**
**Date: 2026-11-09.** This document is a *licensing proposal* and contributor policy, **not itself a copyright license**. The public repository must not be represented as legally reusable under an OSI-approved license until a valid, owner-authorized `LICENSE` is committed.

## 1. Permanent project goal

All VirgoStars components intended for future open-source release should be:

1. **Free of license fees:** anyone can download, inspect, modify, run, build upon and redistribute the original project under an open-source license, including **commercial use**, with no royalties or API subscription required merely for using the library.
2. **Attribution-preserving:** people who redistribute the library or modified derivatives must preserve legally required notices crediting the **VirgoStars project** and applicable authors/contributors.
3. **Transparent:** include license texts, sources, provenance, attribution, third-party notices, change notices and documentation of any exceptions.
4. **Decoupled from the paid product:** VirgoStars web platform, private server, user data, credentials, proprietary brand assets and shipped Astrolabio hardware are **not automatically included** in this public commitment.

**Important legal nuance:** OSI-approved open-source licenses allow third parties to charge for *their copies, hosting or services*. We can commit to distributing the **official VirgoStars software at no charge**, but we cannot simultaneously demand that *all downstream copies remain free of charge* while describing the license as OSI-compliant. See https://opensource.org/osd.

## 2. Recommended default for CODE: Apache License 2.0 + NOTICE

**Preferred default for original software owned or licensable by VirgoStars** (pending review of third-party code and author ownership):

- `LICENSE`: full unmodified Apache License, Version 2.0;
- `NOTICE`: original project attribution and URL, plus legally required notices for included works;
- README: explain mandatory preservation of notices on redistribution and provide an optional visible on-screen credit pattern;
- retain headers, license references, copyright notices and modified-file notices where required.

Apache 2.0 permits free commercial use and distribution, and Section 4 requires preservation of specified copyright, patent, trademark and attribution notices in redistributed source/derivatives. If a NOTICE file exists, an appropriate readable copy of the applicable notices must be preserved in a permitted place (NOTICE, documentation/source, or an appropriate in-product third-party notice display). **Apache 2.0 does NOT generally mandate a visible "Powered by VirgoStars" badge in every product interface.** See https://www.apache.org/licenses/LICENSE-2.0 and https://www.apache.org/legal/apply-license.

**Suggested attribution record**, once legal names and repo URLs are confirmed:

```text
This software includes code from the VirgoStars open-source project.
Project: VirgoStars
Repository: https://github.com/Cesare-ml/<relevant-repository>
Copyright: <verified rights holder(s)>
License: Apache License, Version 2.0
```

For apps that want a visible acknowledgment, a footer/about/credits entry such as:

```text
Built with VirgoStars — https://github.com/Cesare-ml/virgostars-ephemeris
```

is **recommended**, but would not be made mandatory by Apache alone.

## 3. If ON-SCREEN attribution is a strict requirement

The phrase "the project must receive credit" can refer to two significantly different obligations:

- **Redistribution attribution**: the app/repo distribution retains copyright and NOTICE information; Apache 2.0 is suitable and widely adopted for code.
- **Visible end-user UI attribution**: every qualifying application must display "VirgoStars" (for example on startup or in an About screen). A different legally suitable license is needed, such as the OSI-approved **Common Public Attribution License 1.0 (CPAL-1.0)** with correctly completed Exhibit B, after legal review and compatibility assessment: https://opensource.org/license/cpal-1.0.

CPAL is materially different from Apache 2.0, less common in the library ecosystem, and may complicate integration and contributor adoption. Do **not** casually add a custom "always display our logo" condition to Apache 2.0 and still claim the code is unmodified Apache-licensed.

**Open owner decision:** Is attribution required only upon redistribution (default Apache 2.0 + NOTICE), or must a visible app UI acknowledgment also be legally mandatory (separate CPAL compatibility review)?

## 4. Graphic assets and documentation: separate licenses and rights reviews

Code and creative artwork must **not** be indiscriminately placed under one license.

For separately identifiable, **original, owned and approved** symbols/diagrams/illustrations and documentation, consider **Creative Commons Attribution 4.0 International (CC BY 4.0)**:

- permits copying, adaptations and commercial reuse without royalty payments;
- requires appropriate attribution, license reference and indication of changes when shared;
- does **not** mandate the same screen location for credit in every application;
- **must never be applied to unreviewed images, fonts or third-party artwork**.

Creative Commons expressly recommends **against CC licenses for software code**. See https://creativecommons.org/faq/ and https://creativecommons.org/licenses/by/4.0/.

For `Cesare-ml/astrological-assets`: all existing review records remain **pending** until the owner confirms the copyright, permissions and desired license *per asset or demonstrably consistent family*. Some images may remain private, excluded or separately licensed. Assets must **not** be auto-published simply because a code repository is open.

For fonts, planetary kernels, astronomical datasets, reference tables and any other third-party material, obey their **own** license, usage and redistribution terms. An openly downloadable dataset is not automatically CC BY. Track the NASA/JPL source, citation and applicable terms separately. The clean-room ephemeris project must not copy Swiss Ephemeris implementation code or assume any third-party license is automatically cleared.

## 5. Scope and release plan

Candidate repositories:

| Repository | 2026-11-09 visibility | Intended subject | Public release gate |
|---|---|---|---|
| `Cesare-ml/virgostars-ephemeris` | Public | Clean-room astronomical engine | Choose code license; confirm provenance of JPL/SPK kernels and sources |
| `Cesare-ml/astrological-functions` | Private | Framework-independent astrology functions | Dependency/provenance/security audit and owner approval before publication |
| `Cesare-ml/astrological-charts` | Private | Angular/Three.js rendering code | Check extracted-code ownership, dependencies and artwork-free package before publication |
| `Cesare-ml/astrological-assets` | Private | Symbols, skin artwork, texture/font files | Per-file/family approvals; may need a **separate private/public split** |

The private VirgoStars production monorepo is **not part of this proposed open-source license grant**. Its website, database, saved natal charts, credentials, paid service, private visuals and existing hardware are not changed.

## 6. Do-not-merge release gates

- [ ] Determine the legally correct copyright holders for original code and contributor contributions.
- [ ] Decide Apache 2.0 + NOTICE versus a license requiring visible UI attribution.
- [ ] Review compatibility of all upstream licenses, especially any historical Swiss Ephemeris-derived code and NASA/JPL data.
- [ ] Add actual complete `LICENSE` and `NOTICE` only **after owner approval**; a policy draft is not a substitute.
- [ ] Add per-asset license metadata and publish only rights-cleared images.
- [ ] Verify README/CONTRIBUTING, redistribution instructions, SPDX metadata and examples.
- [ ] Confirm no secrets, customer data or private application logic are introduced.
- [ ] Keep paid products and production deployments independent; no GitHub Actions needed.

**Working instruction for Codex:** Until the license decision is final, maintain this preference in documentation and prepare compliance files **as draft**, but do not put public-release licenses on code/assets or change repo visibility without explicit owner authorization.
