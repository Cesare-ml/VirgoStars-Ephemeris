# Licensing, credits and data provenance

**Current code license:** [Common Public Attribution License 1.0 (CPAL-1.0)](../LICENSE), with VirgoStars attribution details completed in Exhibits A and B. This applies to original source code published by the initial developer; third-party files retain their own terms. Please read the actual license text rather than relying only on this explanatory guide.

## What developers may do

You may use, study, modify, integrate, commercially distribute, and redistribute this project within CPAL-1.0's conditions. There is **no royalty or license purchase fee** for the open-source code. The official repository is free to access. Third parties may offer paid hosting, products, services or distribution, subject to the license.

This is not a blanket license to use the VirgoStars name/logo for endorsements, to download a production database or to redistribute third-party ephemeris data.

## Required VirgoStars attribution

The **Original Developer** is recorded as `Cesare-ml (VirgoStars project)`. Exhibit B specifies:

```text
Copyright (c) 2026 Cesare-ml (VirgoStars project)
Powered by VirgoStars
https://www.virgostars.it/
```

The attribution requirement is **required for Larger Works**, as specified by the completed Exhibit B. When a GUI application uses CPAL-covered code, see **Section 14**, including the requirement for prominent display at launch or session start. The project recommends a standard **Credits/About/Open Source** screen with a clickable VirgoStars link, but merely placing an entry in a menu does **not automatically** satisfy every CPAL condition. The displayed form must satisfy the actual Section 14 notice and prominence requirements. No permanent advertisement or image is required by our Exhibit B.

Example credits entry:

```html
<section aria-labelledby="virgostars-credits">
  <h2 id="virgostars-credits">Credits / Open Source</h2>
  <p>
    Powered by <a href="https://www.virgostars.it/" target="_blank"
      rel="noopener noreferrer">VirgoStars</a>
  </p>
</section>
```

The snippet illustrates a clickable entry, **not** a legal compliance certification for Section 14's launch/session requirement. For headless and CLI programs that do not present a GUI, CPAL-1.0 Section 14 expressly says the display obligation does not apply; source distribution notices still do.

## Changes, notices and network services

- Preserve the CPAL license, source notices (Exhibit A) and applicable contributor notices.
- If you modify covered code, document the modifications and dates as required by Section 3.3.
- Make the CPAL-covered source and modifications available under Sections 3.1–3.2 where applicable.
- CPAL Section 15 treats certain network uses (**External Deployment**) like distribution, so making a service accessible online can trigger covered-source obligations even without shipping binaries.
- The license's copyleft applies to the **covered code and modifications**, not automatically to every unrelated file in a larger application. The attribution requirement may nevertheless apply to the **Larger Work** because our Exhibit B selects `required`.
- Keep your own third-party license notices, and do not imply the VirgoStars project endorses your product.

## Original code versus external data

This repository currently contains a small, experimental JavaScript reader for the classic DE441 ephemeris format and synthetic unit tests. It **does not** redistribute any NASA/JPL ephemeris files, Swiss Ephemeris software or Astrodienst datasets.

- Any JPL DE441 `.eph` file is obtained separately under its supplier's conditions; CPAL-1.0 **does not grant rights** in that external file.
- NASA/JPL/NAIF public downloads, NAIF SPICE Toolkit code and Astrodienst distributions have their own provenance and applicable conditions. Assess them separately before bundling or mirroring.
- Third-party libraries added in the future must be reviewed for license compatibility.

The project's code is **not** an endorsement or official NASA, JPL, NAIF or Astrodienst project.

## Contributors and future re-licensing

Contributors keep their copyrights. Submitting a contribution under CPAL does not automatically transfer its ownership to Cesare-ml or guarantee that VirgoStars can relicense everyone's work later.

If a future release changes license, the historical code already distributed under CPAL generally remains usable under that edition's original terms. Any future relicensing must respect ownership, contributor agreements and third-party materials.

## Reference sources

- [Full CPAL-1.0 license in this repository](../LICENSE)
- [Open Source Initiative — CPAL-1.0](https://opensource.org/license/CPAL-1.0)
- [SPDX identifier: CPAL-1.0](https://spdx.org/licenses/CPAL-1.0.html)
- [NOTICE](../NOTICE)

This guide is informative; **the complete license text controls in any conflict**.
