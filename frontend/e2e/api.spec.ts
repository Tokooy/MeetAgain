import { test, expect } from "@playwright/test";
import Ajv2020 from "ajv/dist/2020.js";
import { readFileSync } from "node:fs";

const schema = JSON.parse(
  readFileSync(new URL("../../docs/api.schema.json", import.meta.url), "utf8"),
);
const validator = new Ajv2020({ allErrors: true });
validator.addSchema(schema);
function conforms(name: string, body: unknown) {
  const validate = validator.getSchema(`${schema.$id}#/$defs/${name}`)!;
  expect(validate(body), JSON.stringify(validate.errors)).toBe(true);
}

test("real HTTP responses conform to the confirmed JSON schemas", async ({
  request,
}) => {
  const health = await request.get("/api/health");
  expect(health.status()).toBe(200);
  conforms("HealthResponse", await health.json());
  const graph = await request.get("/api/graph");
  expect(graph.status()).toBe(200);
  conforms("GraphResponse", await graph.json());
  const config = {
    agents: 20,
    ticks: 20,
    seed: "20261116",
    movement: {
      home_move_probability: 0.1,
      away_move_probability: 0.25,
      home_return_probability: 0.6,
    },
    encounters: { include_tick_zero: false, cross_tick_tolerance: 0 },
  };
  conforms("SimulationRequest", config);
  const created = await request.post("/api/simulations", { data: config });
  expect(created.status()).toBe(201);
  const result = await created.json();
  conforms("SimulationResponse", result);
  let cursor: string | null = null;
  let count = 0;
  do {
    const page = await request.get(`/api/simulations/${result.id}/encounters`, {
      params: { limit: 100, ...(cursor ? { cursor } : {}) },
    });
    expect(page.status()).toBe(200);
    const body = await page.json();
    conforms("EncounterPage", body);
    count += body.events.length;
    cursor = body.next_cursor;
  } while (cursor !== null);
  expect(count).toBe(result.summary.encounter_event_count);
  for (const path of [
    "/api/unknown",
    "/api/",
    "/api/simulations/unknown/encounters",
  ]) {
    const response = await request.get(path);
    expect(response.status()).toBe(404);
    conforms("ApiError", await response.json());
  }
  const bad = await request.post("/api/simulations", {
    data: { ...config, agents: 1 },
  });
  expect(bad.status()).toBe(422);
  conforms("ApiError", await bad.json());
  const malformed = await request.post("/api/simulations", {
    data: Buffer.from("{"),
    headers: { "content-type": "application/json" },
  });
  expect(malformed.status()).toBe(400);
  conforms("ApiError", await malformed.json());
  const method = await request.post("/api/health");
  expect(method.status()).toBe(405);
  conforms("ApiError", await method.json());
  const index = await request.get("/");
  expect(index.status()).toBe(200);
  expect(index.headers()["content-type"]).toContain("text/html");
});
