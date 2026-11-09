> **Implementation update (2026-11-09):** `VirgoStars-Ephemeris` now has the complete [CPAL-1.0 LICENSE](../LICENSE) with required VirgoStars attribution for Larger Works, [NOTICE](../NOTICE), and [licensing guide](LICENSING_AND_CREDITS.md) on public `main` via [PR #6](https://github.com/Cesare-ml/VirgoStars-Ephemeris/pull/6). The proposal below remains for the **other future repositories** and must not be read as saying this repository is unlicensed.

# VirgoStars — Free/Open-Source and Attribution Policy (proposed)

**Status: OWNER CHOICE RECORDED — VISIBLE NAME + LINK IN CREDITS/ABOUT. LICENSE TEXT AND ENFORCEABILITY STILL UNDER REVIEW.**
**Date: 2026-11-09.** This document is a *licensing proposal* and contributor policy, **not itself a copyright license**. The public VirgoStars-Ephemeris repository is now licensed under CPAL-1.0; other future repositories need their own legally valid and owner-authorized licenses.

## 1. Permanent project goal

All VirgoStars components intended for future open-source release should be:

1. **Free of license fees:** anyone can download, inspect, modify, run, build upon and redistribute the original project under an open-source license, including **commercial use**, with no royalties or API subscription required merely for using the library.
2. **Attribution-preserving:** people who redistribute the library or modified derivatives must preserve legally required notices crediting the **VirgoStars project** and applicable authors/contributors.
3. **Transparent:** include license texts, sources, provenance, attribution, third-party notices, change notices and documentation of any exceptions.
4. **Decoupled from the paid product:** VirgoStars web platform, private server, user data, credentials, proprietary brand assets and shipped Astrolabio hardware are **not automatically included** in this public commitment.

**Important legal nuance:** OSI-approved open-source licenses allow third parties to charge for *their copies, hosting or services*. We can commit to distributing the **official VirgoStars software at no charge**, but we cannot simultaneously demand that *all downstream copies remain free of charge* while describing the license as OSI-compliant. See https://opensource.org/osd.

## 2. Owner decision: visible project name and link in Credits/About

**Owner preference confirmed 2026-11-09:** Any application with a user-facing GUI integrating a VirgoStars open-source library should provide a user-accessible **Credits / About / Open Source** section that clearly identifies the project and links to it. The intention is modest, useful attribution and indirect discoverability rather than a forced footer or an intrusive startup logo.

**Canonical attribution specification proposed:**

```text
Uses VirgoStars open-source technology
VirgoStars — https://www.virgostars.it/
```

Alternatively an English entry: `Powered by VirgoStars — https://www.virgostars.it/`. The website is a proposed canonical marketing URL; a link to the originating GitHub repository can be included as an optional second reference. Do not assert endorsement of the downstream product by VirgoStars.

**Where:** a conventional Credits/About page or dialog accessible through ordinary navigation. If the product exposes other third-party acknowledgments, VirgoStars should be comparably visible. CLI/headless software cannot have a GUI About screen: require preservation of attribution in distribution documentation/help as legally supported by the final license.

**Important limitation:** An Apache 2.0 NOTICE alone does *not* make an in-app clickable link mandatory. A technically convenient About screen is a **design preference**; its enforceability depends on the final selected license. Do not claim that this specific formulation has already been enacted as a license condition.

## 3. Considered alternative: Apache License 2.0 + NOTICE

**Fallback only, if the owner relaxes the legally mandatory UI-credit requirement** (subject to third-party code and author ownership review):

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

## 4. Licensing options for mandatory on-screen attribution

The phrase "the project must receive credit" can refer to two significantly different obligations:

- **Redistribution attribution**: the app/repo distribution retains copyright and NOTICE information; Apache 2.0 is suitable and widely adopted for code.
- **Visible end-user UI attribution**: every qualifying application must display "VirgoStars" (for example on startup or in an About screen). A different legally suitable license is needed, such as the OSI-approved **Common Public Attribution License 1.0 (CPAL-1.0)** with correctly completed Exhibit B, after legal review and compatibility assessment: https://opensource.org/license/cpal-1.0.

CPAL is materially different from Apache 2.0, less common in the library ecosystem, and may complicate integration and contributor adoption. Do **not** casually add a custom "always display our logo" condition to Apache 2.0 and still claim the code is unmodified Apache-licensed.

**Owner decision now resolved:** visible name and project link in Credits/About is desired **as a binding obligation**, not just source redistribution credits. **Open legal/engineering decision:** which OSI-approved standard license can enforce that goal without creating unacceptable network-source disclosure, integration friction or incompatibilities? We must choose before release.

### CPAL-1.0 specific cautions

The OSI-approved CPAL-1.0 §14 with **Exhibit B completed** is a candidate because it supports attribution information (copyright notice, short phrase, URL and optional image) in a GUI, and allows attribution obligations in **Larger Works** when specifically selected in Exhibit B. Its wording addresses a *prominent display at launch/session start*, while recognizing continued use of an existing About/attribution form as one way to satisfy the notice requirement. **A link buried in a settings page is not automatically sufficient** to establish legal compliance. §15 also treats external/network deployments of original code or modifications as distributions and triggers source availability obligations. Both implications require counsel or knowledgeable license review before choosing CPAL.

The OSI-approved **Attribution Assurance License (AAL)** is another alternative, but it explicitly requires prominent attribution when dependent executables launch and includes signed-text conditions. It is not necessarily a better match for a low-friction About screen. Official sources: https://opensource.org/license/aal and https://opensource.org/license/cpal-1.0.

Do not append a custom 'must display link in About' sentence to an unmodified Apache/MIT license and call that combination standard Apache/MIT or automatically OSI-approved. The LGPL/GPLv3 options also require detailed review; the FSF notes that GPLv3 §7(b) notices do not freely extend to arbitrary promotional links/logos (https://www.fsf.org/blogs/community/gpl-compliant-legal-notices-author-attributions).

## 5. Graphic assets and documentation: separate licenses and rights reviews

Code and creative artwork must **not** be indiscriminately placed under one license.

For separately identifiable, **original, owned and approved** symbols/diagrams/illustrations and documentation, consider **Creative Commons Attribution 4.0 International (CC BY 4.0)**:

- permits copying, adaptations and commercial reuse without royalty payments;
- requires appropriate attribution, license reference and indication of changes when shared;
- does **not** mandate the same screen location for credit in every application;
- **must never be applied to unreviewed images, fonts or third-party artwork**.

Creative Commons expressly recommends **against CC licenses for software code**. See https://creativecommons.org/faq/ and https://creativecommons.org/licenses/by/4.0/.

For `Cesare-ml/astrological-assets`: all existing review records remain **pending** until the owner confirms the copyright, permissions and desired license *per asset or demonstrably consistent family*. Some images may remain private, excluded or separately licensed. Assets must **not** be auto-published simply because a code repository is open.

For fonts, planetary kernels, astronomical datasets, reference tables and any other third-party material, obey their **own** license, usage and redistribution terms. An openly downloadable dataset is not automatically CC BY. Track the NASA/JPL source, citation and applicable terms separately. The clean-room ephemeris project must not copy Swiss Ephemeris implementation code or assume any third-party license is automatically cleared.

## 6. Scope and release plan

Candidate repositories:

| Repository | 2026-11-09 visibility | Intended subject | Public release gate |
|---|---|---|---|
| `Cesare-ml/virgostars-ephemeris` | Public | Clean-room astronomical engine | Choose code license; confirm provenance of JPL/SPK kernels and sources |
| `Cesare-ml/astrological-functions` | Private | Framework-independent astrology functions | Dependency/provenance/security audit and owner approval before publication |
| `Cesare-ml/astrological-charts` | Private | Angular/Three.js rendering code | Check extracted-code ownership, dependencies and artwork-free package before publication |
| `Cesare-ml/astrological-assets` | Private | Symbols, skin artwork, texture/font files | Per-file/family approvals; may need a **separate private/public split** |

The private VirgoStars production monorepo is **not part of this proposed open-source license grant**. Its website, database, saved natal charts, credentials, paid service, private visuals and existing hardware are not changed.

## 7. Do-not-merge release gates

- [ ] Determine the legally correct copyright holders for original code and contributor contributions.
- [x] Owner chose visible **VirgoStars name + hyperlink** in an accessible Credits/About UI as the desired condition.
- [ ] Conduct a license compatibility/legal review (CPAL-1.0 §14/§15 vs AAL vs permissive Apache-2.0 + *voluntary* visible credits) and choose an OSI-approved license that truly meets the desired UI rule.
- [ ] Confirm the final link destination and legally correct copyright holder(s), and determine whether the same rule should apply to headless/CLI distribution in documentation/help.
- [ ] Review compatibility of all upstream licenses, especially any historical Swiss Ephemeris-derived code and NASA/JPL data.
- [ ] Add actual complete `LICENSE` and `NOTICE` only **after owner approval**; a policy draft is not a substitute.
- [ ] Add per-asset license metadata and publish only rights-cleared images.
- [ ] Verify README/CONTRIBUTING, redistribution instructions, SPDX metadata and examples.
- [ ] Confirm no secrets, customer data or private application logic are introduced.
- [ ] Keep paid products and production deployments independent; no GitHub Actions needed.

See [license decision record](LICENSE_DECISION_RECORD.md) and the [non-binding CPAL Exhibit B example](CPAL_EXHIBIT_B_WORKING_DRAFT.md) before choosing. **The owner has not yet approved CPAL or any code license.**

**Working instruction for Codex:** Implement the suggested Credits/About snippet in demos/documentation as a working example without representing it as an already enforceable license. Until the license decision is legally/technically reviewed, prepare license exhibits/compliance files **as drafts**, but do not put a public-release LICENSE on code/assets or change repository visibility without explicit owner authorization.
