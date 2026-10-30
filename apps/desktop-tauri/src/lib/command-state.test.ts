import assert from "node:assert/strict";
import test from "node:test";
import { visibleCommandState, type KeyedLoadable } from "./command-state.ts";

test("same-key command reload retains its last good value", () => {
  const key = 'audio_overview:{"projectId":"A"}';
  const snapshot: KeyedLoadable<{ source: string }, string> = {
    key, value: { status: "ready", data: { source: "A-private" } },
  };
  assert.deepEqual(visibleCommandState(key, snapshot), snapshot.value);
});

test("switching project or command never renders a previous response", () => {
  const old: KeyedLoadable<{ source: string }, string> = {
    key: 'audio_overview:{"projectId":"A"}',
    value: { status: "ready", data: { source: "A-private" } },
  };
  for (const key of [
    'audio_overview:{"projectId":"B"}',
    'data_sources:{"projectId":"A"}',
    null,
  ]) {
    assert.deepEqual(visibleCommandState(key, old), { status: "loading" });
  }
});

test("stale error responses are hidden under a different request key", () => {
  const old: KeyedLoadable<string, { kind: string; message: string }> = {
    key: 'project_detail:{"projectId":"A"}',
    value: { status: "error", error: { kind: "denied", message: "A-only" } },
  };
  assert.deepEqual(visibleCommandState('project_detail:{"projectId":"B"}', old), { status: "loading" });
  assert.deepEqual(visibleCommandState(old.key, old), old.value);
});
