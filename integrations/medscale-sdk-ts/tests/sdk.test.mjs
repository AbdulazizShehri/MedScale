import assert from "node:assert/strict";
import { createHash, webcrypto } from "node:crypto";
import { test } from "node:test";
import {
  JOURNEY_WORKFLOW_RESOURCE_TYPES,
  MedScaleApiError,
  MedScaleClient,
  WorkflowClientError,
} from "../dist/index.js";

globalThis.crypto ??= webcrypto;

const BASE_URL = "http://127.0.0.1:8000";
const DIGEST_A = "sha256:" + "a".repeat(64);
const DIGEST_B = "sha256:" + "b".repeat(64);
const DIGEST_C = "sha256:" + "c".repeat(64);
const DIGEST_D = "sha256:" + "d".repeat(64);
const REQ_ID = "req_" + "1".repeat(32);

test("MedScaleClient rejects blank baseUrl", () => {
  assert.throws(() => new MedScaleClient({ baseUrl: "   ", fetch: async () => new Response("{}") }));
});

test("MedScaleClient analyze, deidentify, journeyWorkflow, and API error decoding", async () => {
  const calls = [];
  const client = new MedScaleClient({
    baseUrl: BASE_URL,
    fetch: async (url, init) => {
      calls.push({ url, init });
      if (url.endsWith("/analyze")) {
        return new Response(
          JSON.stringify({
            text: "Synthetic fever note.",
            entities: [{ text: "fever", label: "SIGN_SYMPTOM", confidence: 0.96, start: 10, end: 15, metadata: {} }],
            model_name: "medscale-clinical-ner",
            timestamp: "2026-11-28T00:00:00Z",
            processing_time: 0.012,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        );
      }
      if (url.endsWith("/pii/deidentify")) {
        return new Response(
          JSON.stringify({
            original_text: "Jane Doe",
            deidentified_text: "[NAME]",
            pii_entities: [],
            method: "mask",
            timestamp: "2026-11-28T00:00:00Z",
            num_entities_redacted: 1,
            metadata: {},
            audit_report: null,
          }),
          { status: 200, headers: { "Content-Type": "application/json" } },
        );
      }
      if (url.includes("/v1/journey/resources")) {
        return new Response(JSON.stringify({ resources: [], next_cursor: null }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        });
      }
      return new Response(
        JSON.stringify({ error: { code: "validation_error", message: "Synthetic validation refusal", details: { field: "text" } } }),
        { status: 422, headers: { "Content-Type": "application/json" } },
      );
    },
  });

  const ner = await client.analyze({ text: "Synthetic fever note." });
  assert.equal(ner.entities.length, 1);
  assert.equal(ner.entities[0].label, "SIGN_SYMPTOM");

  const deid = await client.deidentify({ text: "Jane Doe", method: "mask" });
  assert.equal(deid.deidentified_text, "[NAME]");
  assert.equal(deid.num_entities_redacted, 1);

  await client.journeyWorkflow("cohort", { first: 5 });
  assert.ok(calls[2].url.includes(`resource_type=${JOURNEY_WORKFLOW_RESOURCE_TYPES.cohort}`));

  await assert.rejects(
    client.ground({ text: "" }),
    (err) => err instanceof MedScaleApiError && err.status === 422 && err.code === "validation_error",
  );
});

test("GovernedWorkflowClient validates snapshot digests and detects binding mismatches", async () => {
  const previewDigest = "sha256:" + createHash("sha256").update("medscale.service.workflow_preview.v1\0[]").digest("hex");
  const runId = "run_" + "a".repeat(32);
  const baseSnapshot = {
    schema_version: "medscale.service.workflow_response.v1",
    run_id: runId,
    workflow_id: "workflow:medscale.local/prior-auth@1.0.0",
    action_digest: DIGEST_A,
    state_digest: DIGEST_B,
    phase: "waiting-review",
    effects: [],
    preview_digest: previewDigest,
    proposed_effect_count: 0,
    committed_effect_count: 0,
    outcome: null,
    receipt_digest: null,
    cancellation_requested: false,
  };

  const client = new MedScaleClient({
    baseUrl: BASE_URL,
    fetch: async (url) => {
      const payload = url.endsWith("/status")
        ? { ...baseSnapshot, action_digest: DIGEST_D }
        : baseSnapshot;
      return new Response(JSON.stringify(payload), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    },
  });

  const snap = await client.workflowPreflight({
    schema_version: "medscale.service.workflow_request.v1",
    run_id: runId,
    workflow_id: "workflow:medscale.local/prior-auth@1.0.0",
    action_digest: DIGEST_A,
    expected_state_digest: null,
    request_id: REQ_ID,
  });
  assert.equal(snap.phase, "waiting-review");

  await assert.rejects(
    client.workflowStatus({
      schema_version: "medscale.service.workflow_request.v1",
      run_id: runId,
      workflow_id: "workflow:medscale.local/prior-auth@1.0.0",
      action_digest: DIGEST_A,
      expected_state_digest: null,
      request_id: null,
    }),
    (err) => err instanceof WorkflowClientError && err.code === "workflow_binding_mismatch",
  );
});
