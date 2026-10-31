import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { PINNED_COMMIT, canonical, loadVenue, readJson, root, sha256, venues, verifySourceSnapshot } from '../compat/voicegis/harness';
import { legacySearchReference, projectLegacyGraph, projectLegacyOptions, projectLegacyPolicyPackage, projectLegacyResult } from '../compat/voicegis/native-search';
import { createCompiledBuildingRuntime } from '../compat/voicegis/upstream/src/data/compiledBuilding';
import { calculateCompiledRoute } from '../compat/voicegis/upstream/src/engine/compiledRoutePolicy';
import { resolveOperationalOverlay, type OperationalOverlay } from '../compat/voicegis/upstream/src/engine/operationalOverlay';
import { buildLegacySearchCases } from './legacy-search-cases';
import { buildLegacyPolicyRouteCases, type PolicyBatch } from './legacy-policy-route-cases';

type Query = { queryId: string; startId: string; destinationId: string; options: ReturnType<typeof projectLegacyOptions> };
type Request = ReturnType<typeof projectLegacyGraph> & { protocolVersion: string; sourceCommit: string; requestId: string; queries: Query[] };
type Expected = ReturnType<typeof projectLegacyResult> & { queryId: string };
type Batch = { request: Request; expected: Expected[] };
type Baseline = { id: string; startId: string; destinationId: string; profile: 'standard' | 'wheelchair'; sha256: string };

assert.equal(verifySourceSnapshot(), 35);
const baseline = readFileSync(resolve(root, 'expected/route-digests.jsonl'), 'utf8').trim().split('\n').map(line => JSON.parse(line) as Baseline);
assert.equal(baseline.length, 7_238);
assert.equal(new Set(baseline.map(item => item.id)).size, baseline.length);
const batches: (Batch | PolicyBatch)[] = [];
const envelope = (requestId: string) => ({ protocolVersion: '0.1.0', sourceCommit: PINNED_COMMIT, requestId });
let checkedJourneys = 0;
for (const [venueIndex, venue] of venues.entries()) {
  const pkg = await loadVenue(venue);
  const runtime = createCompiledBuildingRuntime(pkg);
  const rows = baseline.filter(item => item.id.startsWith(`${venue}/`));
  assert.equal(rows.length, [242, 2496, 200, 4300][venueIndex], venue);
  const batch: Batch = { request: { ...envelope(venue), ...projectLegacyGraph(runtime.routingNodes, runtime.routingEdges), queries: [] }, expected: [] };
  for (const item of rows) {
    const original = calculateCompiledRoute(runtime, item.startId, item.destinationId, { profile: item.profile });
    // The complete legacy result must still match its original fingerprint
    // before extracting the smaller search-only native expectation.
    assert.equal(sha256(canonical(original)), item.sha256, item.id);
    const options = projectLegacyOptions({ accessibleOnly: item.profile === 'wheelchair' });
    const expected = projectLegacyResult(original);
    assert.deepEqual(legacySearchReference(runtime.routingNodes, runtime.routingEdges, item.startId, item.destinationId, options), expected, item.id);
    batch.request.queries.push({ queryId: item.id, startId: item.startId, destinationId: item.destinationId, options });
    batch.expected.push({ queryId: item.id, ...expected });
    checkedJourneys++;
  }
  if (venue === 'asterion-medical-center') {
    const overlay = readJson<OperationalOverlay>('upstream/buildings/asterion-medical-center/operations/all-public-lifts-closed.overlay.json');
    const evaluatedAt = '2026-07-22T12:00:00.000Z';
    const resolved = resolveOperationalOverlay(overlay, pkg, evaluatedAt);
    assert.equal(resolved.valid, true);
    assert.ok(resolved.closedEdgeIds.length > 0);
    for (const profile of ['standard', 'wheelchair'] as const) {
      const queryId: string = `${venue}/lift-outage/${profile}`;
      const startId = 'poi:poi-main-entrance', destinationId = 'poi:poi-cardiology';
      const original = calculateCompiledRoute(runtime, startId, destinationId, { profile, operationalOverlay: overlay, evaluatedAt });
      const expected = projectLegacyResult(original);
      assert.equal(expected.outcome, profile === 'standard' ? 'routed' : 'no-route', queryId);
      const options = projectLegacyOptions({ accessibleOnly: profile === 'wheelchair', closedEdgeIds: resolved.closedEdgeIds });
      assert.deepEqual(legacySearchReference(runtime.routingNodes, runtime.routingEdges, startId, destinationId, options), expected, queryId);
      batch.request.queries.push({ queryId, startId, destinationId, options });
      batch.expected.push({ queryId, ...expected });
    }
  }
  batches.push(batch);
  // Exercise the complete original journey inventory through the policy
  // protocol with explicit absence, independently of the retained v1 calls.
  batches.push({ request: {
    ...batch.request, protocolVersion: '0.2.0', requestId: `${venue}/policy-none`, package: projectLegacyPolicyPackage(pkg),
    queries: batch.request.queries.slice(0, rows.length).map(query => ({
      queryId: query.queryId, startId: query.startId, destinationId: query.destinationId,
      options: { accessibleOnly: query.options.accessibleOnly, allowRestricted: query.options.allowRestricted }, policy: { mode: 'none' },
    })),
  }, expected: batch.expected.slice(0, rows.length) });
}
assert.equal(checkedJourneys, baseline.length);
for (const item of buildLegacySearchCases()) {
  batches.push({
    request: { ...envelope(item.id), nodes: item.nodes, edges: item.edges, queries: [{ queryId: item.id, ...item.request }] },
    expected: [{ queryId: item.id, ...item.expected }],
  });
}
batches.push(...await buildLegacyPolicyRouteCases());
assert.equal(batches.length, 36);
const queryCount = batches.reduce((count, batch) => count + batch.request.queries.length, 0);
assert.equal(queryCount, 14_630);

const executable = process.env.WAYFORM_LEGACY_SEARCH_BIN ?? resolve(process.env.CARGO_TARGET_DIR ?? 'target', 'debug', process.platform === 'win32' ? 'wayform-legacy-search.exe' : 'wayform-legacy-search');
const run = spawnSync(executable, [], {
  input: batches.map(batch => JSON.stringify(batch.request)).join('\n') + '\n',
  encoding: 'utf8', timeout: 60_000, maxBuffer: 64 * 1024 * 1024,
});
assert.ifError(run.error);
assert.equal(run.status, 0, run.stderr);
assert.equal(run.stderr, '');
const responses = run.stdout.trim().split('\n').map(line => JSON.parse(line) as {
  protocolVersion: string; sourceCommit: string; requestId: string; outcome: string; results: Expected[];
});
assert.equal(responses.length, batches.length);
for (const [index, actual] of responses.entries()) {
  const { request, expected } = batches[index];
  assert.equal(actual.protocolVersion, request.protocolVersion, request.requestId);
  assert.equal(actual.sourceCommit, PINNED_COMMIT, request.requestId);
  assert.equal(actual.requestId, request.requestId);
  assert.equal(actual.outcome, 'completed', JSON.stringify(actual));
  assert.equal(actual.results.length, expected.length, request.requestId);
  for (const [queryIndex, result] of actual.results.entries()) {
    assert.deepEqual(result, expected[queryIndex], expected[queryIndex].queryId);
  }
}
console.log(`Native legacy conformance: ${checkedJourneys} pinned journeys under both protocols, 2 resolved lift-outage queries, 26 search cases and 126 native policy-routing queries passed (${queryCount} queries, ${batches.length} batches). Package verification, directions and legacy receipts remain in TypeScript.`);
