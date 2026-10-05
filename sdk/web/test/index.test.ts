import { test } from "node:test";
import assert from "node:assert/strict";
import { isGuardBrowser, requestGuardToken, verifyGuardToken, GuardUnavailableError } from "../src/index.ts";

const jsonResponse = (status: number, body: unknown) =>
  (async () => new Response(JSON.stringify(body), { status })) as typeof fetch;

test("detects guard host", async () => {
  const scope = { __GUARD__: { version: 1, getToken: async (n: string) => `tok-${n}` } };
  assert.equal(isGuardBrowser(scope), true);
  assert.equal(await requestGuardToken("abc", scope), "tok-abc");
});

test("throws when guard host is missing", async () => {
  assert.equal(isGuardBrowser({}), false);
  await assert.rejects(requestGuardToken("abc", {}), GuardUnavailableError);
});

test("passes through server decision", async () => {
  const r = await verifyGuardToken("t", { endpoint: "http://x", fetch: jsonResponse(200, { decision: "allow", score: 93 }) });
  assert.deepEqual(r, { decision: "allow", score: 93 });
});

test("falls back to challenge on rejection, bad body, or network error", async () => {
  const opts = { endpoint: "http://x" };
  assert.equal((await verifyGuardToken("t", { ...opts, fetch: jsonResponse(401, { error: "x" }) })).decision, "challenge");
  assert.equal((await verifyGuardToken("t", { ...opts, fetch: jsonResponse(200, { decision: "nuke" }) })).decision, "challenge");
  const failing = (async () => { throw new Error("down"); }) as typeof fetch;
  assert.equal((await verifyGuardToken("t", { ...opts, fetch: failing })).decision, "challenge");
});
