# VirgoStars — Credits/About implementation contract (proposed)

**Owner decision: 2026-11-09.** Future open-source VirgoStars projects should be free of official licensing fees and require **visible attribution with the VirgoStars name and project hyperlink** within a normal Credits/About/Open Source screen of downstream GUI applications.

**This is an implementation and release specification, not itself an enforceable copyright license.** The legal obligation must be expressed under a lawfully chosen OSI-approved license or by a separately agreed contract that does not falsely claim standard Apache/MIT terms. No repository is automatically re-licensed by this document.

## 1. Human-readable credit

**Minimum visible entry (proposed):**

```text
Uses VirgoStars open-source technology
VirgoStars — https://www.virgostars.it/
```

Alternative compact text: **Powered by VirgoStars** linking to **https://www.virgostars.it/**.

The link should be an accessible, functioning hyperlink to the original project. The linked destination can be updated by the owner before release. A second link to the exact originating GitHub repository is welcome, but the official VirgoStars brand should be identifiable.

A link alone without the project name is insufficient for the intended credit. A project name alone without a link is also insufficient for the intended credit.

No automatic endorsement, official affiliation, certification or partnership claim is granted.

## 2. Placement and presentation

- GUI products: provide an ordinary **Credits / About / Open Source** screen or dialog reachable through navigation.
- Identify **VirgoStars** alongside any other technology/library credits; do not intentionally bury it in low-contrast, hidden, inaccessible or machine-only metadata.
- Avoid mandatory oversized logos, popup advertising or a fixed full-screen watermark; the owner prefers an unobtrusive, authentic acknowledgment.
- For mobile or web apps a plain-text hyperlink is sufficient from a UX perspective; do not demand a specific CSS styling, permanent footer or a logo.
- For native apps, link handling should open the user-appropriate browser.
- CLI/headless/backend applications: there may be no GUI. Use a human-accessible AUTHORS/NOTICE, CLI `--about` or `--credits`, documentation and distribution notices, consistent with the final license. Do not pretend the GUI obligation applies to a program without GUI.
- Preserve legally required license/NOTICE attribution in redistributed copies in addition to the in-app credit.

## 3. Technical acceptance tests for a GUI demo

When Codex adds an Angular demonstration application:

1. Add an accessible Credits/About panel whose visible text includes **VirgoStars**.
2. The clickable link must point to the approved project URL and have a descriptive accessible name (not only "click here").
3. Verify navigation with keyboard; preserve accessible contrast.
4. Prefer implementation in the **host demo**, not injected globally into a reusable library that has no knowledge of its host UI.
5. Add a small DOM/component test verifying the name and link presence; keep all actual licensing/attribution claims distinct from the test.
6. The test checks that **our example** follows the preferred pattern; it does **not** prove that arbitrary downstream applications will satisfy a license, nor does it substitute for legal review.

## 4. License compatibility decision

The owner wants a visible credit to be **obligatory**, not merely encouraged.

- Apache-2.0 + NOTICE by itself enforces distribution notices, **not** any UI About entry.
- CPAL-1.0 (§14 and a completed Exhibit B) is OSI-approved and provides a potential visible GUI attribution duty; it also covers **Larger Works** if Exhibit B specifies so, and §15 adds network-deployment/source-availability obligations. Its launch/session language must be reviewed: an About link that is never actually displayed may not necessarily satisfy every circumstance.
- The Attribution Assurance License (AAL) is another OSI-approved attribution license, but it uses launch-time displays and signed text and is not a straightforward low-impact About-only solution.
- GPLv3 §7(b) author-attribution notices are limited; do not assume they can mandate promotional URL links merely because an About menu exists.
- If we choose a more permissive Apache-2.0 or MIT license, a GUI credit can be **requested and implemented voluntarily**, but cannot be inaccurately advertised as legally mandatory under that standard license.
- A custom licensing clause is **not automatically OSI-approved**, and may cause compatibility issues.

Read authoritative license texts before making the decision:
- https://opensource.org/license/cpal-1.0
- https://opensource.org/license/aal
- https://www.fsf.org/blogs/community/gpl-compliant-legal-notices-author-attributions

**Required review before final publication:** identify copyright owners and contributors, choose an appropriate license, check third-party dependencies and data rights, correctly complete license exhibits, assess source availability implications and obtain owner authorization.

## 5. Coverage boundaries

The proposal covers software **authored or lawfully licensed by VirgoStars** and explicitly released under the finalized license. The VirgoStars production website/backend, real user data, private Astrolabio tooling and hardware, unreviewed images/fonts and NASA/JPL kernels do **not** become free/open-licensed by implication.

Keep the chart artwork and fonts under per-asset rights review. Any eventual CC BY 4.0 license for owned graphics has its own attribution terms and does not substitute for a code library's UI-credit license.

## 6. Agent constraints

**Codex must**:
- place a normal Credits/About acknowledgment in the independent Angular chart demo;
- add tests for the example's name/link and document the intended license;
- report actual third-party licensing conflicts and keep open-source publication gated.

**Codex must not**:
- create a `LICENSE` file declaring CPAL or another binding license before owner approval;
- modify private repository visibility, npm publication, AWS production or deployed Astrolabio devices;
- add GitHub Actions just for attribution tests;
- claim that a README guideline alone legally binds all downstream applications.
