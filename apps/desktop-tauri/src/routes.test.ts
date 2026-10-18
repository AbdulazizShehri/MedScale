import assert from "node:assert/strict";
import test from "node:test";
import { areas, isRouteId, routes, searchRoutes } from "./routes.ts";

test("the closed registry preserves all 25 current desktop routes", () => {
  assert.equal(routes.length, 25);
  assert.equal(new Set(routes.map((route) => route.id)).size, 25);
  assert.ok(routes.every((route) => areas.includes(route.area)));
  assert.ok(isRouteId("Patients"));
  assert.equal(isRouteId("patients"), false);
  assert.equal(isRouteId("../Patients"), false);
});

test("palette matching is bounded to route metadata", () => {
  assert.equal(searchRoutes("fhir")[0]?.id, "Exports");
  assert.equal(searchRoutes("command center")[0]?.id, "Home");
  assert.deepEqual(searchRoutes("delete patient"), []);
  assert.deepEqual(searchRoutes("x".repeat(129)), []);
});
