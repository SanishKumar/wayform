import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { inputDigests } from '../packages/mobility-contract/src/identity';
import type { EvaluationContext, MobilityBundle, MobilityProfile, PathAssessment } from '../packages/mobility-contract/src/types';
import { validateBundle, validateContext, validateProfile } from '../packages/mobility-contract/src/validate';
import { addObservation, addPolicy, fixture } from '../tests/fixtures';

type Data = { bundle: MobilityBundle; profile: MobilityProfile; context: EvaluationContext };
type Outcome = 'routed' | 'insufficient-evidence' | 'infeasible';
type Scenario = Data & { id: string; startId: string; destinationId: string; outcome: Outcome; edgeIds?: string[] };
const document = JSON.parse(readFileSync(new URL('../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json', import.meta.url), 'utf8')) as {
  synthetic: boolean;
  examples: (Data & { id: string; request: { startId: string; destinationId: string; edgeIds: string[] }; expected: PathAssessment })[];
};
assert.equal(document.synthetic, true);
assert.equal(document.examples.length, 10);
const scenarios: Scenario[] = document.examples.map(item => {
  assert.deepEqual(assessPath(item.bundle, item.profile, item.context, item.request.startId, item.request.destinationId, item.request.edgeIds), item.expected, item.id);
  // Reversed supplied paths are invalid, but searching between those valid
  // endpoints is a well-formed query with no route in this directed graph.
  if (item.expected.status === 'invalid') assert.equal(item.id, 'one-way-reversed');
  const outcome: Outcome = item.expected.status === 'suitable' ? 'routed' : item.expected.status === 'unknown' ? 'insufficient-evidence' : 'infeasible';
  return { ...item, ...item.request, outcome, edgeIds: outcome === 'routed' ? item.request.edgeIds : undefined };
});

function detour(): Data {
  const data = fixture();
  data.bundle.edges.push(
    { ...data.bundle.edges[0], id: 'ac', to: 'c', lengthMeters: 20 },
    { ...data.bundle.edges[0], id: 'cb', from: 'c', lengthMeters: 20 },
  );
  addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'ac');
  addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'cb');
  return data;
}
const additional: { id: string; change(data: Data): void; edgeIds: string[] }[] = [
  { id: 'suitable-shortcut', change() {}, edgeIds: ['ab'] },
  { id: 'narrow-shortcut', change(data) { data.bundle.observations[0].value = 0.6; }, edgeIds: ['ac', 'cb'] },
  { id: 'unknown-shortcut', change(data) { data.bundle.observations = data.bundle.observations.filter(item => item.edgeId !== 'ab'); }, edgeIds: ['ac', 'cb'] },
  { id: 'closed-shortcut', change(data) { addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; }, edgeIds: ['ac', 'cb'] },
  { id: 'preferred-detour', change(data) { data.bundle.edges[0].kind = 'lift'; data.profile.preferences = { avoidKinds: ['lift'], penaltyMeters: 50 }; }, edgeIds: ['ac', 'cb'] },
];
for (const item of additional) {
  const data = detour();
  item.change(data);
  scenarios.push({ ...data, id: item.id, startId: 'a', destinationId: 'b', outcome: 'routed', edgeIds: item.edgeIds });
}
scenarios.push({ ...fixture(), id: 'stationary', startId: 'a', destinationId: 'a', outcome: 'routed', edgeIds: [] });

const requests = scenarios.flatMap(item => {
  const { bundle, profile, context } = item;
  assert.deepEqual(validateBundle(bundle), [], item.id);
  assert.deepEqual(validateProfile(profile), [], item.id);
  assert.deepEqual(validateContext(context, bundle), [], item.id);
  const request = {
    protocolVersion: '0.1.0', requestId: item.id,
    snapshot: inputDigests(bundle, profile, context),
    nodes: bundle.nodes.map(node => node.id),
    edges: bundle.edges.map(({ id, from, to, lengthMeters }) => ({ id, from, to, lengthMeters })),
    // This intentionally uses the public supplied-path oracle. Revalidating the
    // bundle per edge is acceptable for this small conformance corpus, not a
    // production bulk-evaluation API.
    decisions: bundle.edges.map(edge => {
      const assessment = assessPath(bundle, profile, context, edge.from, edge.to, [edge.id]);
      assert.notEqual(assessment.status, 'invalid', `${item.id}/${edge.id}`);
      return { edgeId: edge.id, suitability: assessment.status, penaltyMeters: profile.preferences.avoidKinds.includes(edge.kind) ? profile.preferences.penaltyMeters : 0 };
    }),
    startId: item.startId, destinationId: item.destinationId,
  };
  return [request, { ...request, requestId: `${item.id}-permuted`, nodes: [...request.nodes].reverse(), edges: [...request.edges].reverse(), decisions: [...request.decisions].reverse() }];
});

const executable = process.env.WAYFORM_SEARCH_BIN ?? resolve(process.env.CARGO_TARGET_DIR ?? 'target', 'debug', process.platform === 'win32' ? 'wayform-search.exe' : 'wayform-search');
const run = spawnSync(executable, [], { input: requests.map(item => JSON.stringify(item)).join('\n') + '\n', encoding: 'utf8', timeout: 30_000, maxBuffer: 4 * 1024 * 1024 });
assert.ifError(run.error);
assert.equal(run.status, 0, run.stderr);
assert.equal(run.stderr, '');
const responses = run.stdout.trim().split('\n').map(line => JSON.parse(line) as {
  protocolVersion: string; requestId: string; snapshot: ReturnType<typeof inputDigests>;
  outcome: string; edgeIds?: string[]; distanceMeters?: number; costMeters?: number;
});
assert.equal(responses.length, requests.length);
for (const [index, response] of responses.entries()) {
  const item = scenarios[Math.floor(index / 2)], request = requests[index];
  assert.equal(response.protocolVersion, request.protocolVersion, item.id);
  assert.equal(response.requestId, request.requestId, item.id);
  assert.deepEqual(response.snapshot, request.snapshot, item.id);
  assert.equal(response.outcome, item.outcome, item.id);
  if (item.outcome !== 'routed') {
    assert.equal(response.edgeIds, undefined, item.id);
    continue;
  }
  assert.deepEqual(response.edgeIds, item.edgeIds, item.id);
  const assessment = assessPath(item.bundle, item.profile, item.context, item.startId, item.destinationId, response.edgeIds!);
  assert.equal(assessment.status, 'suitable', item.id);
  assert.equal(response.distanceMeters, assessment.receipt.distanceMeters, item.id);
  assert.equal(response.costMeters, assessment.receipt.preferenceCostMeters, item.id);
}
console.log(`Native search conformance: ${scenarios.length} synthetic scenarios, ${requests.length} requests passed. Evidence and receipts are evaluated by TypeScript.`);
