# B6 drafts for legal review

**DRAFTS. NOT IN EFFECT. NOT LEGAL ADVICE.** These are engineering drafts that give counsel a factual starting point for blocker B6 in `docs/beta/REAL_DATA_READINESS.md`. Nothing here is published, shown in the app, or binding until the founder and counsel approve a final text. `REAL_PHI_AUTHORIZATION` stays `NOT_AUTHORIZED`.

| Draft | Purpose |
|---|---|
| [INTENDED_USE_DRAFT.md](INTENDED_USE_DRAFT.md) | What MedScale is for and not for, written so it can be checked against medical-device rules |
| [PRIVACY_NOTICE_DRAFT.md](PRIVACY_NOTICE_DRAFT.md) | What data the app and the website handle, where it goes, and what MedScale never receives |
| [TERMS_OF_USE_DRAFT.md](TERMS_OF_USE_DRAFT.md) | Terms for the app and the website, alongside the Apache-2.0 licence |

Every factual statement links to the code or document it comes from, so counsel can check it. Where a statement depends on a decision nobody has made yet, the draft says so in **[DECISION]** brackets.

## Questions for counsel

1. **Medical device status.** Do the intended-use statement and the current features (evidence coverage, conflicts, cited answers, local model runs) keep MedScale outside medical-device regulation in the jurisdictions the founder targets (for example SFDA in Saudi Arabia, the EU MDR, the US FDA's clinical decision support guidance)? Which features or words would change that?
2. **Who is the controller or processor.** MedScale runs entirely on the user's computer and the project receives no data. Does that make the user or their organization the only controller (PDPL, GDPR) or covered entity (HIPAA), with the project as neither controller nor processor?
3. **Jurisdictions and lawful basis.** Which jurisdictions must the notice address first, and what must it say about lawful basis when an organization uses MedScale with real records?
4. **Website.** The website is a static site on GitHub Pages with no analytics or cookies of its own. What must its notice say about GitHub's own server logs?
5. **Open-source terms.** Can the terms rely on Apache-2.0's warranty disclaimer and limitation of liability, or does a health product need its own terms on top?
6. **Model and catalog content.** Models are third-party and downloaded only on the user's request. What must the terms say about third-party models and their licences?
7. **Before real data.** What must exist before the founder authorizes real patient data: a DPIA, a data-processing agreement template for organizations, breach-notification procedures, or something else?
