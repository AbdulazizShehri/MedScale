# MedScale product state language

**Authority:** Spec 095; native StateBadge in components.slint.

| State | Non-color marker | Meaning and response |
|---|---|---|
| Current | [=] | Bound to the current revision; still separate from clinical authority. |
| Reviewed | [+] | An explicit review exists; show its source/receipt where available. |
| Unknown | [?] | No supported conclusion; inspect missing source/coverage. Never infer absence or success. |
| Stale | [~] | Source/revision/freshness has changed; refresh or re-evaluate before use. |
| Conflicting | [!] | Sources or expectations disagree; show both sides and preserve provenance. |
| Partial | [%] | Only part of scope is supported; explain the missing part. |
| Denied | [x] | Authority refused the operation; show the refusal/recovery path. |
| Unavailable | [-] | A needed session/backend/resource cannot currently be used; do not imply capability. |
| Unsupported | [/] | This implementation does not support the requested operation/format. |
| Unmeasured | [:] | No qualification/measurement supports the claim; do not fabricate a metric. |
| Corrupt | [#] | Integrity validation failed; stop affected use and expose recovery/evidence. |

Literal names are the semantic authority; markers supplement them, never replace them. Each state has a distinct accessible name. Optional functional colors supplement text only; default presentation is neutral. Unknown is the fallback for an unrecognized presentation state. Existing backend lifecycle states remain exact and are not reclassified by substring or generalized into success/error.

The compact StatusPill compatibility wrapper displays the supplied existing lifecycle label without inferring success. New route views use an explicit StateBadge state and adjacent source/limitation details. Foundation checks verify the eleven names/markers and live-token contrast; actual native state rendering and platform assistive technology qualification are recorded separately.
