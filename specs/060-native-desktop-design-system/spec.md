# Feature Specification: Native Desktop Design System + Command Center

**Feature Branch**: `spec/060-native-desktop-design-system`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: FOUNDER_SUPPLIED_FINAL_V0_DIRECTION + POST_TRUSTED_V1_PRODUCT_PHASE

## Goal
Replace the thin non-WebView Desktop scaffold with the first real native MedScale application surface using the founder-approved visual direction, while preserving the Core Host authority model, local-first privacy, synthetic-only data policy, and honest release claims.

## User Stories

### US1 — Native MedScale shell
A user launches `medscale-desktop` and receives a real native window with MedScale branding, global navigation, search/command entry, local/privacy status, and a Home/Command Center surface.

### US2 — Founder-approved design system
The shell implements canonical tokens/components from `DESIGN.md`: original purple MedScale M, Cohere-inspired supporting colors, calm light surfaces, restrained density, and consistent keyboard/focus behavior.

### US3 — Synthetic command-center data
Until real-PHI authorization exists, the launch surface uses deterministic synthetic/demo data only. It must never imply that fixture metrics are real production analytics.

### US4 — Accessible native semantics
Primary navigation, search, quick actions, and status surfaces expose accessible roles/labels through the native Slint accessibility path and support keyboard navigation. This unit does not claim WCAG conformance.

### US5 — Existing qualification compatibility
`--smoke` and `--perf-idle-ms` remain headless/bounded and continue supporting package/performance CI. No WebView/Tauri is introduced.

## Requirements
- **FR-001**: Desktop runtime MUST be native and non-WebView.
- **FR-002**: Use Slint 1.16.1 because it matches workspace MSRV 1.88, supports Windows/macOS/Linux, and exposes OS accessibility integration.
- **FR-003**: UI code MUST NOT open canonical storage, keys, or network clients directly.
- **FR-004**: Desktop visual tokens and interaction rules MUST be centralized and reusable.
- **FR-005**: Normal launch opens the UI; `--smoke` and `--perf-idle-ms` MUST NOT require a display server/window.
- **FR-006**: Home/Command Center MUST include navigation, global search, quick actions, system/privacy state, metrics/queue/activity placeholders backed only by deterministic synthetic data in this unit.
- **FR-007**: Command palette keyboard shortcut MUST be part of the shell architecture; command execution may be stubbed to safe navigation/actions in 060.
- **FR-008**: Accessibility labels/roles MUST exist on primary controls. WCAG conformance remains false until final qualification.
- **FR-009**: External release gates for signing, qualified hardware, and macOS signed-product qualification remain unchanged.
- **FR-010**: Mobile implementation remains deferred until Desktop + CLI launch.

## Success Criteria
- Native Desktop opens on macOS locally and compiles in the existing three-OS CI matrix.
- `cargo test --workspace --locked` and clippy remain green.
- Existing desktop smoke/perf tests remain green.
- No `tauri`/WebView dependency enters the workspace.
- Design-system regression tests verify canonical tokens/assets and shell labels.
- Doctor reports native Slint shell and final-v0 implementation present, while `wcag_conformance_claimed=false` and `release_ready=false`.
- Visual acceptance checklist records alignment with `DESIGN.md` and founder-approved direction.

## Anti-scope
Patient-detail feature completeness; Workflow Studio execution; full Insights analytics; CLI parity expansion; mobile apps; real PHI; production signing; notarization; WCAG certification; claiming qualified-hardware performance. Those are sequenced in 061+.
