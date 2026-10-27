import assert from 'node:assert/strict';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { bundleDigest } from '../packages/mobility-contract/src/identity';
import { validateBundle, validateContext, validateProfile } from '../packages/mobility-contract/src/validate';
import { addObservation, addPolicy, fixture } from '../tests/fixtures';

type Data = ReturnType<typeof fixture>;
function chain(data: Data) {
  data.bundle.edges.push({ ...data.bundle.edges[0], id: 'bc', from: 'b', to: 'c', lengthMeters: 12, kind: 'ramp' });
  addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'bc');
}
export function buildPathCases() {
  const cases: { id: string; status: string; change?(data: Data): void; start?: string; end?: string; edges?: string[] }[] = [
    { id: 'stationary', status: 'suitable', end: 'a', edges: [] },
    { id: 'unknown-start', status: 'invalid', start: 'missing' },
    { id: 'unknown-destination', status: 'invalid', end: 'missing' },
    { id: 'stationary-unknown', status: 'invalid', start: 'missing', end: 'missing', edges: [] },
    { id: 'no-steps-to-other-node', status: 'invalid', edges: [] },
    { id: 'unknown-edge', status: 'invalid', edges: ['missing'] },
    { id: 'empty-edge-id', status: 'invalid', edges: [''] },
    { id: 'unfinished', status: 'invalid', end: 'c' },
    { id: 'valid-chain', status: 'suitable', change: chain, end: 'c', edges: ['ab', 'bc'] },
    { id: 'chain-out-of-order', status: 'invalid', change: chain, end: 'c', edges: ['bc', 'ab'] },
    { id: 'invalid-after-valid-prefix', status: 'invalid', change: chain, end: 'c', edges: ['ab', 'ab'] },
    { id: 'preferences', status: 'suitable', change(data) { chain(data); data.profile.preferences = { avoidKinds: ['ramp'], penaltyMeters: 7 }; }, end: 'c', edges: ['ab', 'bc'] },
    { id: 'unknown-second-edge', status: 'unknown', change(data) { chain(data); data.bundle.observations.pop(); }, end: 'c', edges: ['ab', 'bc'] },
    { id: 'unsuitable-dominates-unknown', status: 'unsuitable', change(data) { chain(data); data.bundle.observations.pop(); data.bundle.observations[0].value = 0.5; }, end: 'c', edges: ['ab', 'bc'] },
    { id: 'unused-policy', status: 'suitable', change(data) { chain(data); addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; }, start: 'b', end: 'c', edges: ['bc'] },
    { id: 'untrusted-policy-still-recorded', status: 'unknown', change(data) { addPolicy(data.bundle, data.context); data.context.trustedIssuerIds = []; } },
    { id: 'policy-ids-utf16-order', status: 'suitable', change(data) {
      addPolicy(data.bundle, data.context);
      for (const id of ['\u{10000}', '\ue000', '2', '10', '__proto__']) {
        data.bundle.policyRequirements.push({ id, edgeIds: ['ab'] });
        Object.defineProperty(data.context.minimumPolicyRevisions, id, { value: 3, enumerable: true, writable: true, configurable: true });
        data.context.policies.push({ ...data.context.policies[0], requirementId: id });
      }
      for (const policy of data.context.policies) policy.bundleDigest = bundleDigest(data.bundle);
    } },
    { id: 'cycle-repeats-reasons', status: 'unknown', change(data) {
      data.bundle.edges.push({ ...data.bundle.edges[0], id: 'ba', from: 'b', to: 'a' });
      data.bundle.observations = [];
    }, edges: ['ab', 'ba', 'ab'] },
    { id: 'distance-overflow', status: 'invalid', change(data) { chain(data); for (const edge of data.bundle.edges) edge.lengthMeters = Number.MAX_VALUE; }, end: 'c', edges: ['ab', 'bc'] },
    { id: 'preference-overflow', status: 'invalid', change(data) { data.bundle.edges[0].lengthMeters = Number.MAX_VALUE; data.profile.preferences = { avoidKinds: ['walkway'], penaltyMeters: Number.MAX_VALUE }; } },
    { id: 'subprecision-path-sum', status: 'suitable', change(data) { chain(data); data.bundle.edges[0].lengthMeters = 2 ** 54; data.bundle.edges[1].lengthMeters = 1; }, end: 'c', edges: ['ab', 'bc'] },
    { id: 'unicode-reason-order', status: 'unknown', change(data) {
      chain(data); data.bundle.edges[0].id = '\ue000'; data.bundle.edges[1].id = '\u{10000}'; data.bundle.observations = [];
    }, end: 'c', edges: ['\ue000', '\u{10000}'] },
  ];
  return cases.map(item => {
    const data = fixture(); item.change?.(data);
    assert.deepEqual(validateBundle(data.bundle), [], item.id);
    assert.deepEqual(validateProfile(data.profile), [], item.id);
    assert.deepEqual(validateContext(data.context, data.bundle), [], item.id);
    const request = { startId: item.start ?? 'a', destinationId: item.end ?? 'b', edgeIds: item.edges ?? ['ab'] };
    const expected = assessPath(data.bundle, data.profile, data.context, request.startId, request.destinationId, request.edgeIds);
    assert.equal(expected.status, item.status, item.id);
    return { id: item.id, synthetic: true, ...data, request, expected };
  });
}
