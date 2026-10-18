# MedScale Product Truth

## Product
MedScale is a local-first healthcare intelligence workspace for clinicians, care teams, operators, and technical builders. Desktop and CLI are the launch surfaces. Mobile applications come only after Desktop + CLI launch quality is established.

## Core promise
MedScale turns trusted longitudinal health data into understandable, source-aware workflows without weakening authority, privacy, or evidence boundaries.

## Launch surfaces
- **Desktop**: the primary visual product for daily clinical and operational work.
- **CLI**: a first-class, scriptable product with the same authority and capability model as Desktop.
- **Mobile**: deferred until Desktop + CLI are launched; no mobile work is part of Specs 060-067 unless explicitly promoted later.

## Product principles
1. Local-first and private by default.
2. One Rust authority path for Desktop, CLI, and future clients.
3. Evidence and provenance are visible product concepts, not hidden implementation detail.
4. AI may assist, summarize, and propose; authority-changing actions remain explicit.
5. Fast enough to feel immediate, calm enough to support consequential work.
6. Advanced power is discoverable through command palette, keyboard, and CLI rather than visual clutter.
7. Synthetic/permitted fixtures only until REAL_PHI authorization is explicitly granted.

## Experience target
The product should feel precise, quiet, fast, inspectable and unmistakably MedScale. The current authority is the founder-approved paired-M identity, Inter, JetBrains Mono NL and a black/white system with restrained sky-blue interaction and orange attention accents (founder decision 2026-10-07; status never relies on color alone). Dark mode uses layered near-black surfaces; light mode uses white/near-white surfaces with black hierarchy. Semantic states retain explicit text. Earlier Geist and multicolor/gradient identities (including the superseded #125 direction) remain superseded; the 2026-10-07 accents are the only approved colors beyond black and white.

Models are a first-class product concept. Users must be able to see what model is installed/admitted, where it came from, what task it serves, what runtime/device executes it, its provenance/trust state, benchmark state, and promotion/rollback state. Hugging Face may be a governed model source, but never becomes an authority plane.

Competitive evidence is also a first-class product concept. The product may show where MedScale has a proven advantage over OpenMed and must equally show where OpenMed remains ahead. “MedScale beats OpenMed” is forbidden unless the exact capability has a bound comparative result.

MedScale is a **Clinical Intelligence OS** with the operating principle **Evidence first. Action second.** The 2026-09-30 founder decision selects staged Tauri 2 + React/TypeScript/Vite/Tailwind presentation migration over the existing Rust Core. Slint remains the functional reference until required parity, native/privacy evidence and a separate retirement decision. Spec 095 retains its Slint-era brand meaning; the new shell starts as a synthetic-only preparatory 096 package.

Canonical identity rules live under `docs/brand/`. Product surfaces must follow `The work is the interface`: clinical context and evidence come before dashboards, decorative AI chrome, or product metrics.

## User jobs
Desktop and CLI must ultimately cover the same trusted product capabilities: status/privacy inspection, vault/open lifecycle, ingest/import, patient/subject search, longitudinal timeline, brief/summary, coverage, documents/labs, evidence/insights, care-plan workflows, tasks/messages where authorized, audit, exports, backup/recovery, integrations, packs, model/runtime inspection, comparative evidence, and operator diagnostics.

## Non-goals for this phase
- No real PHI authorization.
- No product runtime cloud dependency.
- No Tauri/WebView private-data readiness claim before the open privacy qualification gate is resolved.
- No fake user sign-in: local workspace/vault access does not establish organizational authentication.
- No mobile app implementation before Desktop + CLI launch.
- No claim of WCAG conformance until final product qualification evidence exists.
- No production signing/notarization claims without real credentials.

## Success definition
The Desktop and CLI should feel like two expressions of one product: same data semantics, same authority rules, same vocabulary, same privacy posture, and equivalent power appropriate to each interaction model.

## Product differentiation sequence (2026-09-15)
Specs 068–072 are freshly promoted after the founder rejected the prior UI/product positioning. The sequence rebuilds identity and information architecture, admits a real bounded local model runtime and governed Hugging Face pack path, productizes Model Center and OpenMed Evidence Center, and then requalifies the resulting release candidate. Until this sequence closes, `MEDSCALE_IMPLEMENTATION_COMPLETE=false` is the honest state.
