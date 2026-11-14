# Feature Specification: Desktop + CLI Hardening

**Feature Branch**: `spec/066-desktop-cli-hardening`
**Created**: 2026-09-15
**Status**: CLOSED_CANONICAL
**Promotion**: POST_SPEC_065_PRODUCT_HARDENING_SEQUENCE

## Goal
Harden the Desktop+CLI launch surface by removing misleading placeholder/risk copy, closing the remaining top-level Documents route, enforcing a usable native minimum window, and reducing duplicate PR CI without weakening required gates.

## Requirements
- **FR-001**: Every top-level Desktop navigation route MUST render an honest native surface or explicit recognized boundary; no advertised route may fall through to a future-slice placeholder.
- **FR-002**: Home synthetic operational content MUST NOT invent clinical risk ranking or look like real patient data.
- **FR-003**: Documents MUST expose custody/intake, extraction, and authority separation; admitted custody MUST NOT imply understanding.
- **FR-004**: Care-plan copy MUST preserve review-first semantics and MUST NOT imply silent generation/commit.
- **FR-005**: Global search/command copy MUST match implemented behavior and MUST NOT imply an unavailable general assistant.
- **FR-006**: Native Desktop MUST set a usable minimum window while preserving keyboard focus/accessibility semantics.
- **FR-006A**: Text tokens used on primary light surfaces MUST meet an engineering contrast floor of 4.5:1; semantic status MUST remain readable without relying on accent color alone. This is not a WCAG conformance claim.
- **FR-007**: CI MAY remove redundant `spec/**` push runs only if pull-request qualification and post-merge `main` qualification remain intact with the same six required job names.
- **FR-008**: CLI authority, existing patient/Insights/workflow/utility contracts, headless probes, and portable-package qualification MUST remain intact.
- **FR-009**: No real PHI, MESC work, clinical risk model, production signing/notarization, WCAG conformance, or release-readiness claim is authorized.

## Success Criteria
- No `High risk` or real-looking patient names remain in Home synthetic demo content.
- Documents is a real native route with explicit non-claims.
- All named nav routes have dedicated route handling and no generic future-slice fallback.
- Window minimum is encoded and native UI still compiles.
- Primary light-surface text tokens pass the 4.5:1 engineering contrast regression and status labels do not rely on accent color alone.
- CI triggers exactly one PR workflow plus one post-merge `main` workflow instead of duplicate branch+PR workflows.
- Hardening regressions, Desktop/CLI tests, Clippy, MSRV checks, and exact-head CI pass before canonical closure.
