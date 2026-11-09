# License decision record — mandatory GUI credits (draft, not binding)

**Status:** Legal/design decision candidate. **No license has been granted by this file.** Updated 2026-11-09.

## Project requirement

All future VirgoStars libraries that are explicitly open-sourced should be free to use in official distribution, including commercial projects, **while providing a legally enforceable acknowledgement of the project name and a URL in a visible Credits/About area** where the integrated product has a user-facing graphical interface.

Example proposed credit:

> **Powered by VirgoStars** — https://www.virgostars.it/

The desired placement is **Credits / About / Open Source**, without intrusive banners or a permanent watermark. For CLI/headless code, preserve the applicable required notices in documentation/help. **Do not assume every OSI-approved license supports this exact narrow rule.**

## License comparison

| Model | Legally mandatory GUI project link? | Allows commercial reuse? | Other relevant obligations | Suitability |
|---|---|---|---|---|
| MIT/BSD-2 | No; notice obligations mainly in copies/distributions | Yes | Preserve copyright/license notices | High adoption; insufficient for mandatory GUI credit |
| Apache-2.0 + NOTICE | No guaranteed in-app link; notices may be carried by documentation and distribution | Yes | Copyright, patent, NOTICE and changes obligations | Good library compatibility; only *voluntary* UI credit |
| CC BY 4.0 for creative content | Reasonable attribution; cannot rigidly fix exact About placement | Yes | Credit, license link, modifications indication when sharing | Possible only for approved images/docs, not code |
| **CPAL-1.0 + Exhibit B** | **Potentially yes**, if properly configured to cover Larger Works | Yes | GUI §14 *prominence at launch/session*, and §15 network-source obligations | Closest existing OSI option, but stronger than just having an About link |
| Attribution Assurance License (AAL) | Attribution can be obligatory, especially on launch | Yes | Launch-time signed-text/other terms | Less suitable to unobtrusive About-only UX |

**Why CPAL-1.0 cannot be rubber-stamped:** the actual §14 refers to **prominent display at launch or start of session**. It recognizes continued use of an existing About display, but merely providing a clickable About link that the user never visits is not necessarily enough. Section 15 treats certain network deployments as distribution, requiring source availability for covered code. This creates more legal and compliance work for integrators than Apache.

CPAL-1.0 is an OSI-approved license; Apache-2.0 is also OSI-approved, but adding a custom UI-credit clause to Apache does not make the modified license the standard OSI-approved Apache 2.0.

### Practical working proposal

**Current recommendation to review with qualified open-source licensing counsel**: CPAL-1.0 + completed Exhibit B (including the Larger Works field) for self-owned source code *only if* we accept its actual attribution and network deployment requirements.

An alternate product strategy (if easy third-party adoption matters more): Apache-2.0 + NOTICE, plus a clearly documented and shipped Credits/About example requested by the project — but **no promise** it will legally force every app to display a link.

These are a deliberate trade-off. No license document should be merged until the copyright owner selects the trade-off with the actual terms understood.

## Exhibit B parameters requiring review

- **Copyright notice:** the verified legal name(s) of the copyright holder(s), date/year and developer identity.
- **Phrase:** `Powered by VirgoStars` (3 words, within CPAL's maximum 10-word phrase).
- **URL:** `https://www.virgostars.it/`.
- **Image:** none required; the owner prefers unobtrusive text/link.
- **Larger Works:** to reach third-party applications that integrate the library, **required** must be considered; choosing `not required` would undermine the specific project goal.
- **GUI display:** CPAL §14's prominent launch/session requirement must be observed. A conventional About menu alone may **not** be enough. UI examples should not claim legal compliance without review.

See the [working Exhibit B draft](CPAL_EXHIBIT_B_WORKING_DRAFT.md).

## Dependency / provenance matrix before choosing

| Repository | Licensable software? | Special gates |
|---|---|---|
| `virgostars-ephemeris` (public, docs/design only) | No blanket reuse grant yet; no `LICENSE` on main | Clean-room provenance and NASA/JPL SPK/NAIF data terms, copyright ownership, patent/algorithm review |
| `astrological-functions` (private) | Still not released | Remove any copied Swiss Ephemeris code, review dependencies and algorithm sources |
| `astrological-charts` (private) | Still not released | Angular/Three.js/GSAP/RxJS transitive licenses; check authorship and extraction from VirgoStars |
| `astrological-assets` (private) | Code tools separable from artwork | Explicit per-file rights, font/texture licenses; never blanket-CPAL an unreviewed image |

Do not publish the private VirgoStars production platform by implication. Do not claim NASA/JPL datasets become CPAL/CC-BY merely because this repo links to them.

## No-fee clarification

OSI open-source code may be resold by others. VirgoStars may provide its official upstream implementation and documentation **without charge**, and may sell separate hosted services, support or hardware. A prohibition against all third-party sales would be incompatible with the open-source objective.

## Final acceptance checklist

- [x] Owner wants VirgoStars name + URL accessible to product users, preferably under Credits/About.
- [ ] Owner approves **CPAL-1.0's actual prominent display and §15 network obligations**, or chooses a more permissive license with merely encouraged UI credit.
- [ ] Legal/compatibility reviewer approves Exhibit A and B for the intended copyright owners and integrations.
- [ ] Choose one normative project URL; confirm legal owner(s) and GitHub organizational authority.
- [ ] Audit third-party licences and files.
- [ ] Add official *unaltered* license text and correctly completed exhibits to the approved code repos.
- [ ] Add NOTICE/THIRD_PARTY_NOTICES and a small working About/Credits example with tests.
- [ ] Confirm license identifiers, packaging and release metadata for npm consumers.
- [ ] Owner approves public release / repository visibility separately.

**References**
- OSI CPAL license text: https://opensource.org/license/cpal-1.0
- SPDX CPAL-1.0: https://spdx.org/licenses/CPAL-1.0.html
- Apache-2.0: https://www.apache.org/licenses/LICENSE-2.0
- CC BY 4.0 attribution: https://creativecommons.org/licenses/by/4.0/
- OSI Open Source Definition: https://opensource.org/osd
