# MedScale Hugging Face — publication / verification

## Source of truth
- `space/README.md` and `space/index.html` → `MedScale/MedScale`
- `organization/README.md` and `organization/index.html` → `MedScaleAI/README`
- `PROFILE-FIELDS.md` → suggested human-account and organization settings (requires the site UI)

These are **marketing and documentation assets**, not the MedScale desktop application, and not a claim that the Hugging Face Space is a clinical product or AI inference endpoint.

## Verified in source preparation
- Static Space SDK metadata; no GPU or paid compute needed.
- Interactive site-style desktop and 390-pixel mobile layout checked locally: zero horizontal overflow.
- Primary entry points include working public source destinations: download, product guide, GitHub, research Space and dataset.
- No invented customer names, clinical performance outcomes, certifications, or attribution of external models to MedScale.
- No external JavaScript dependency, trackers, signup forms or collection of patient data.

## Before sign-off
1. Run `python marketing/huggingface/publish.py --dry-run`.
2. Confirm a write-authorized Hugging Face session **as @MedScale**; never paste any secret into a public repository or chat.
3. Publish using `python marketing/huggingface/publish.py` or the equivalent authenticated browser interface.
4. View both published URLs, including the org's `Organization Card`. Confirm images, links and static rendering.
5. Check 390px, 768px and desktop widths in an actual online browser, verify badge and logo alignment, and check screen-reader landmarks.
6. Verify `https://medscale.si/download/` still offers only the currently authorized beta, with the clear synthetic-data-only warning.
7. Update user/org bio and avatar separately via Hugging Face profile settings. No claim of verification or Pro status.
8. Do not modify the SafeOCR Space or dataset during this marketing publication.

## Future updates
Review product claims against `docs/qualification/EXTERNAL_GATES.md` on every release; do not automatically change language from beta to clinically qualified. Third-party model catalogue counts are not first-party model releases.
