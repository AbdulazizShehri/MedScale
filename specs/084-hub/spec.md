# Spec 084 — MedScale Hub Foundation

**Status:** `PROMOTED_IMPLEMENTATION_AUTHORIZED`
**Promoted:** 2026-09-24
**Base SHA:** see `docs/planning/SPEC_084_PROMOTION.md`
**Target branch:** `spec/084-hub`
**Dependency:** 074 + 076 + 079 (all closed).
**Promotion authority:** `docs/planning/SPEC_084_PROMOTION.md`

## 1. Problem

Spec 076 gives each vault rooms, threads, messages, tasks and notes, but
only on one machine. A lab needs several people's devices to work on the
same rooms, including while disconnected, without a MedScale cloud and
without a second authority that could apply changes Core would refuse.

## 2. Goal

A Hub is a MedScale vault whose Core accepts signed submissions from
enrolled devices of other vaults:
- enrollment by one-time invitation, proving possession of an ed25519
  device key;
- a handshake that opens an ordinary Core session bound to the device's
  own Hub participant, granted only Hub sync;
- each submission applied through the Spec 076 authority as that
  participant, so membership, scope and revision checks and the activity
  trail are the existing ones;
- idempotent, ordered submissions, Q07 conflicts, revocation, and a
  per-Project hash-chained event log that clients mirror and verify.

## 3. Scenarios

1. The operator creates a Project room, opens a thread, gives the vault
   its Hub role and invites two laptops. Each laptop joins with its code;
   the operator adds both participants to the room.
2. Both laptops queue a message, a task and a note while offline. After
   each syncs, the Hub has applied both sets in order, and both mirrors
   hold the same verified event list.
3. Both laptops edit the same task and note at revision 1. The first
   applies; the second gets a task conflict and a note conflict copy.
   Messages never conflict.
4. A replayed envelope returns its first outcome and changes nothing. A
   forged, re-signed or out-of-order envelope is refused and recorded.
5. The operator revokes a laptop. Its session ends, its next handshake is
   refused, and the other laptop mirrors the revocation.
6. A device of Project P cannot write a room of Project Q even if added as
   a member; a device that is not a room member cannot write it.
7. The Hub and the clients reopen and continue. A restored client keeps
   its link and mirror but not its device key, and must re-enroll.

## 4. Requirements

The frozen acceptance requirements are in the promotion document. The
contract, storage, security and migration details are in the sibling
files of this folder.

## 5. Out of scope

Network transport, TLS, cloud, federation, multi-Hub, end-to-end
encryption, object transfer, deployment packaging, sync of non-076
objects, approvals and room administration over the Hub, Desktop surface,
real PHI.
