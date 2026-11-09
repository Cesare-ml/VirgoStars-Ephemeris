# CPAL-1.0 Exhibit B — working configuration only (NOT a license)

**DO NOT COPY THIS FILE TO `LICENSE`, `LICENSE.txt`, or a package's legal metadata.** This is only a proposal for legal and owner review; it does not grant reuse rights or modify the official CPAL text.

The [OSI-approved CPAL-1.0](https://opensource.org/license/cpal-1.0) contains a standard Exhibit B with fields for attribution information, including a requirement decision for Larger Works. To implement the owner’s desired public VirgoStars credit across applications incorporating the libraries, the fields below are proposed.

## Suggested field values for official Exhibit B

```text
Attribution Copyright Notice:
  [TO VERIFY: actual licensable copyright owner(s) and relevant year(s)]

Attribution Phrase (not exceeding 10 words):
  Powered by VirgoStars

Attribution URL:
  https://www.virgostars.it/

Graphic Image as provided in the Covered Code, if any:
  None proposed

Display of Attribution Information is [required/not required]
in Larger Works which are defined in the CPAL as a work which
combines Covered Code or portions thereof with code not
governed by the terms of the CPAL:
  required [PROPOSED; LEGAL REVIEW REQUIRED]
```

**Caution about legal scope:** choosing `required` is essential *if* the intent is to reach host apps containing code licensed under other terms. It can add friction for otherwise commercial proprietary consumers, and CPAL-1.0 §15 may require covered source availability for network deployments. The requirement in §14 calls for prominent display at launch/session start, not merely an About hyperlink that could remain unseen. A real release must comply with the actual license, not this short proposal.

## Suggested example UX (not a legal substitute)

Application Credits/About page:

```html
<section aria-labelledby="open-source-credits-title">
  <h2 id="open-source-credits-title">Open-source credits</h2>
  <p>Powered by <a href="https://www.virgostars.it/"
    target="_blank" rel="noopener noreferrer">VirgoStars</a></p>
</section>
```

For an Angular application, keep the credit in the *host demo* and test the visible text, URL and keyboard accessibility. A headless reusable library cannot create its host application's About screen.

## Mandatory pre-release review

1. Confirm who legally owns the code and who is the CPAL Original Developer.
2. Review CPAL §§ 14 and 15, including display on start/session and network deployment obligations.
3. Decide whether this still meets the owner's intention of an unobtrusive Credits/About acknowledgment.
4. Check compatibility with dependency licenses and customer integrations.
5. Only after approval use the official CPAL-1.0 text with complete Exhibits A and B; no invented custom clauses.
6. Add repo-specific copyright and third-party notices. Do not license third-party fonts/artwork/NASA-JPL data without permission.

If the intended UX must be **exclusively About-only with no prominent session-start display**, investigate whether *any existing OSI-approved license* reliably creates exactly that narrow obligation; do not assert CPAL-1.0 does so.
