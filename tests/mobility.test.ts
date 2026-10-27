import assert from 'node:assert/strict';
import { test } from 'node:test';
import { assessPath, localToEnu } from '../packages/mobility-contract/src/assess';
import { validateBundle, validateContext, validateProfile } from '../packages/mobility-contract/src/validate';
import { adaptPinnedVenue } from '../compat/voicegis/adapter';
import { bundleDigest } from '../packages/mobility-contract/src/identity';
import { addEntrance, addObservation, addPolicy, fixture } from './fixtures';
import type { MobilityBundle } from '../packages/mobility-contract/src/types';

const assess = ({ bundle, profile, context }: ReturnType<typeof fixture>, edgeIds = ['ab'], start = 'a', end = 'b') => assessPath(bundle, profile, context, start, end, edgeIds);
test('a measured width meeting the declared limit yields suitability with evidence and a receipt', () => {
  const data = fixture();
  assert.deepEqual(validateBundle(data.bundle), []);
  const result = assess(data);
  assert.equal(result.status, 'suitable');
  assert.deepEqual(result.evidenceIds, ['survey']);
  assert.equal(result.receipt.distanceMeters, 10);
  assert.deepEqual(result.receipt.edgeIds, ['ab']);
  assert.match(result.receipt.bundleDigest!, /^[0-9a-f]{64}$/);
});
for (const mode of ['absent', 'explicit-null', 'unknown-error'] as const) test(`missing evidence remains unknown: ${mode}`, () => {
  const data = fixture();
  if (mode === 'absent') data.bundle.observations = [];
  if (mode === 'explicit-null') Object.assign(data.bundle.observations[0], { value: null, uncertainty: null, unknownReason: 'not-measured' });
  if (mode === 'unknown-error') data.bundle.observations[0].uncertainty = null;
  assert.equal(assess(data).status, 'unknown');
});
test('unknown and known insufficient width are distinct; uncertainty cannot be averaged away', () => {
  const data = fixture();
  data.bundle.observations[0].value = 0.7;
  assert.equal(assess(data).status, 'unsuitable');
  data.bundle.observations[0].value = 0.8;
  assert.equal(assess(data).status, 'unknown');
  data.bundle.observations[0].value = 0.82;
  assert.equal(assess(data).status, 'suitable');
  data.bundle.observations[0].value = 0.82 - 1e-8;
  assert.equal(assess(data).status, 'unknown');
});
test('conflicting fresh measurements stay unknown regardless of array order or a newer timestamp', () => {
  const data = fixture();
  data.bundle.evidence.push({ ...structuredClone(data.bundle.evidence[0]), id: 'other-survey', observedAt: '2026-10-25T10:10:00.000Z' });
  data.bundle.observations.push({ ...data.bundle.observations[0], id: 'conflict', value: 0.6, evidenceId: 'other-survey' });
  const first = assess(data);
  assert.equal(first.status, 'unknown');
  assert.ok(first.reasons.some(reason => reason.code === 'contradictory-evidence'));
  data.bundle.observations.reverse(); data.bundle.evidence.reverse();
  assert.deepEqual(assess(data), first);
});
test('an authored true flag cannot satisfy a physical width requirement', () => {
  const data = fixture();
  data.bundle.observations = [];
  data.bundle.edges[0].legacy = { accessible: true, meaning: 'author-assertion', sourceCommit: 'a'.repeat(40), packageHash: 'b'.repeat(64), sourceEdgeId: 'legacy' };
  assert.equal(assess(data).status, 'unknown');
  data.bundle.edges[0].legacy.accessible = false;
  assert.equal(assess(data).status, 'unsuitable');
});
for (const mutation of ['stale', 'future', 'pending', 'rejected', 'unaccepted', 'expiry', 'future-review'] as const) test(`measurement cannot qualify a path with ${mutation} evidence`, () => {
  const data = fixture();
  const evidence = data.bundle.evidence[0];
  if (mutation === 'stale') evidence.observedAt = '2026-10-20T10:00:00.000Z';
  if (mutation === 'future') { evidence.observedAt = '2026-10-26T10:00:00.000Z'; evidence.review.reviewedAt = '2026-10-26T11:00:00.000Z'; }
  if (mutation === 'future-review') evidence.review.reviewedAt = '2026-10-26T11:00:00.000Z';
  if (mutation === 'pending' || mutation === 'rejected') evidence.review.status = mutation;
  if (mutation === 'unaccepted') evidence.source = 'inference';
  if (mutation === 'expiry') evidence.validUntil = data.context.evaluatedAt;
  assert.equal(assess(data).status, 'unknown');
});
test('fresh accepted evidence can replace stale information without treating stale measurements as current conflicts', () => {
  const data = fixture();
  data.bundle.evidence.push({ ...structuredClone(data.bundle.evidence[0]), id: 'old', observedAt: '2026-10-01T10:00:00.000Z' });
  data.bundle.observations.push({ ...data.bundle.observations[0], id: 'old-width', value: 0.5, evidenceId: 'old' });
  assert.equal(assess(data).status, 'suitable');
});
test('one-way passages cannot be traversed in reverse', () => {
  const data = fixture();
  assert.equal(assess(data, ['ab'], 'b', 'a').status, 'invalid');
});
test('opposite arcs carry distinct slopes and assessments', () => {
  const data = fixture();
  data.profile.hard.maxUphillPercent = 5; data.profile.hard.maxDownhillPercent = 10;
  data.bundle.edges.push({ ...data.bundle.edges[0], id: 'ba', from: 'b', to: 'a' });
  addObservation(data.bundle, 'clearWidthMeters', 1, 0.02, 'ba');
  addObservation(data.bundle, 'slopePercent', 8, 0.2, 'ab');
  addObservation(data.bundle, 'slopePercent', -8, 0.2, 'ba');
  assert.equal(assess(data).status, 'unsuitable');
  assert.equal(assess(data, ['ba'], 'b', 'a').status, 'suitable');
});
test('parallel arcs retain their exact identity and measurements', () => {
  const data = fixture();
  data.bundle.edges.push({ ...data.bundle.edges[0], id: 'narrow-ab', lengthMeters: 2 });
  addObservation(data.bundle, 'clearWidthMeters', 0.5, 0.02, 'narrow-ab');
  assert.equal(assess(data, ['narrow-ab']).status, 'unsuitable');
  assert.equal(assess(data, ['ab']).status, 'suitable');
});
test('a shorter unsuitable path never satisfies hard requirements through cost preferences', () => {
  const data = fixture();
  data.bundle.edges[0].lengthMeters = 1;
  data.bundle.observations[0].value = 0.5;
  data.profile.preferences.penaltyMeters = 1000;
  data.profile.preferences.avoidKinds = ['walkway'];
  assert.equal(assess(data).status, 'unsuitable');
});
test('step-free requirements exclude stairs even before a step-count survey exists', () => {
  const data = fixture(); data.bundle.edges[0].kind = 'stairs'; data.profile.hard.maxStepCount = 0;
  assert.equal(assess(data).status, 'unsuitable');
});
test('surface and door-operation requirements use categorical evidence, including conflicts', () => {
  const data = fixture(); data.bundle.edges[0].kind = 'door';
  data.profile.hard.allowedSurfaces = ['paved']; data.profile.hard.allowedDoorOperations = ['automatic'];
  addObservation(data.bundle, 'surface', 'paved', null); addObservation(data.bundle, 'doorOperation', 'manual', null);
  assert.equal(assess(data).status, 'unsuitable');
  addObservation(data.bundle, 'doorOperation', 'automatic', null);
  assert.equal(assess(data).status, 'unknown');
});
for (const mode of ['missing', 'expired', 'not-yet-active', 'rollback', 'no-floor', 'wrong-bundle', 'wrong-revision', 'untrusted'] as const) test(`required closure information blocks qualification when ${mode}`, () => {
  const data = fixture(); addPolicy(data.bundle, data.context);
  const policy = data.context.policies[0];
  if (mode === 'missing') data.context.policies = [];
  if (mode === 'expired') data.context.evaluatedAt = policy.validUntil;
  if (mode === 'not-yet-active') data.context.evaluatedAt = '2026-10-25T10:59:59.999Z';
  if (mode === 'rollback') policy.revision = 2;
  if (mode === 'no-floor') data.context.minimumPolicyRevisions = {};
  if (mode === 'wrong-bundle') policy.bundleId = 'other';
  if (mode === 'wrong-revision') policy.bundleRevision = 'r0';
  if (mode === 'untrusted') policy.issuerId = 'unknown';
  assert.equal(assess(data).status, 'unknown');
});
test('a current lift closure is unsuitable, expiry becomes unknown, and current reopening is explicit', () => {
  const data = fixture(); data.bundle.edges[0].kind = 'lift'; addPolicy(data.bundle, data.context);
  data.context.policies[0].closedEdgeIds = ['ab'];
  assert.equal(assess(data).status, 'unsuitable');
  data.context.evaluatedAt = data.context.policies[0].validUntil;
  assert.equal(assess(data).status, 'unknown');
  data.context.policies[0].validUntil = '2026-10-25T14:00:00.000Z';
  data.context.policies[0].revision = 4; data.context.minimumPolicyRevisions.operations = 4;
  data.context.policies[0].closedEdgeIds = [];
  assert.equal(assess(data).status, 'suitable');
  assert.deepEqual(assess(data).receipt.policyRevisions, [{ requirementId: 'operations', revision: 4 }]);
});
test('an entrance uses surveyed controls and its node correspondence', () => {
  const data = fixture(); addEntrance(data.bundle);
  assert.equal(assess(data).status, 'suitable');
  data.bundle.transforms[0].rotationDegrees = 0;
  assert.equal(assess(data).status, 'unknown');
  data.bundle.transforms[0].rotationDegrees = 90;
  data.bundle.nodes[1].position[0] += 2;
  assert.equal(assess(data).status, 'unknown');
});
test('overflowing entrance calculations cannot qualify a route', () => {
  const data = fixture(); addEntrance(data.bundle);
  data.bundle.transforms[0].rotationDegrees = Number.MAX_VALUE;
  assert.deepEqual(validateBundle(data.bundle), []);
  const result = assess(data);
  assert.equal(result.status, 'unknown');
  assert(result.reasons.some(reason => reason.code === 'transform-error-exceeds-budget'));
  data.bundle.transforms[0].rotationDegrees = 90;
  assert.equal(assess(data).status, 'suitable');
});
test('entrance uncertainty, provenance and frame identity are required', () => {
  const data = fixture(); addEntrance(data.bundle);
  data.bundle.transforms[0].uncertaintyMeters = 0.5;
  assert.equal(assess(data).status, 'unknown');
  data.bundle.transforms[0].uncertaintyMeters = 0.05;
  data.bundle.evidence[0].source = 'inference';
  assert.equal(assess(data).status, 'unknown');
  data.bundle.edges[0].transformId = null;
  assert.equal(assess(data).status, 'invalid');
});
test('local plan handedness is explicit and transforms do not rewrite geographic coordinates', () => {
  const data = fixture(); addEntrance(data.bundle);
  const point = localToEnu([0, 10, 2], data.bundle.transforms[0], true);
  assert.ok(Math.abs(point[0] - 110) < 1e-9 && Math.abs(point[1] - 200) < 1e-9 && point[2] === 2);
});
test('a disconnected, reversed or incomplete supplied path is invalid', () => {
  const data = fixture();
  for (const ids of [[], ['missing'], ['ab', 'ab']]) assert.equal(assess(data, ids).status, 'invalid');
  assert.equal(assess(data, [], 'a', 'a').status, 'suitable');
});
test('a restrictive result dominates unknown information without erasing its reasons', () => {
  const data = fixture(); data.bundle.edges[0].access = 'restricted'; data.bundle.observations = [];
  const result = assess(data);
  assert.equal(result.status, 'unsuitable');
  assert.ok(result.reasons.some(reason => reason.code === 'missing-measurement'));
});
test('contract validation rejects malformed, duplicate and dangling data', () => {
  const mutations: ((bundle: MobilityBundle) => void)[] = [
    bundle => { bundle.nodes.push(bundle.nodes[0]); },
    bundle => { bundle.edges[0].to = 'missing'; },
    bundle => { bundle.observations[0].evidenceId = 'missing'; },
    bundle => { bundle.observations[0].value = -1; },
    bundle => { bundle.observations[0].value = 'wide'; },
    bundle => { bundle.observations[0].uncertainty = -1; },
    bundle => { bundle.edges[0].lengthMeters = NaN; },
    bundle => { bundle.evidence[0].observedAt = '2026-02-30T10:00:00.000Z'; },
    bundle => { bundle.evidence[0].review.reviewerId = null; },
    bundle => { Object.assign(bundle.edges[0], { accessible: true }); },
    bundle => { addEntrance(bundle); bundle.transforms[0].controls[1] = bundle.transforms[0].controls[0]; },
  ];
  for (const mutate of mutations) { const data = fixture(); mutate(data.bundle); assert.ok(validateBundle(data.bundle).length > 0); assert.equal(assess(data).status, 'invalid'); }
});
test('profile and context validation reject unsafe coercions and duplicate policies', () => {
  const data = fixture();
  assert.ok(validateProfile({ ...data.profile, allowRestricted: 'false' }).length);
  addPolicy(data.bundle, data.context); data.context.policies.push(structuredClone(data.context.policies[0]));
  assert.ok(validateContext(data.context, data.bundle).includes('duplicate-policy'));
  assert.equal(assess(data).status, 'invalid');
});
test('the legacy adapter preserves directional identities and authored exclusions without inventing observations', async () => {
  const bundle = await adaptPinnedVenue('reference-medical-centre');
  assert.deepEqual(validateBundle(bundle), []);
  assert.equal(bundle.observations.length, 0); assert.equal(bundle.evidence.length, 0); assert.equal(bundle.transforms.length, 0);
  const edge = bundle.edges[0], reverse = bundle.edges[1];
  assert.equal(edge.from, reverse.to); assert.equal(edge.to, reverse.from); assert.notEqual(edge.id, reverse.id);
  assert.equal(edge.legacy?.meaning, 'author-assertion');
  const data = fixture();
  const result = assessPath(bundle, data.profile, data.context, edge.from, edge.to, [edge.id]);
  assert.notEqual(result.status, 'suitable');
});
test('assessments are deterministic and do not mutate input data', () => {
  const data = fixture(); addPolicy(data.bundle, data.context);
  const before = structuredClone(data);
  const first = assess(data);
  assert.deepEqual(assess(data), first); assert.deepEqual(data, before);
});
test('receipts identify changed evidence and requirements even if human revision labels are reused', () => {
  const data = fixture();
  const first = assess(data).receipt;
  data.bundle.observations[0].value = 1.1;
  assert.notEqual(assess(data).receipt.bundleDigest, first.bundleDigest);
  data.profile.hard.minClearWidthMeters = 0.9;
  assert.notEqual(assess(data).receipt.profileDigest, first.profileDigest);
  data.context.evaluatedAt = '2026-10-25T12:01:00.000Z';
  assert.notEqual(assess(data).receipt.contextDigest, first.contextDigest);
});
test('a policy cannot authorize changed bundle bytes under a reused revision label', () => {
  const data = fixture(); addPolicy(data.bundle, data.context);
  assert.equal(assess(data).status, 'suitable');
  data.bundle.observations[0].value = 1.1;
  const result = assess(data);
  assert.equal(result.status, 'unknown');
  assert.ok(result.reasons.some(reason => reason.code === 'policy-bundle-mismatch'));
});
test('non-object JSON and malformed path arrays return invalid without throwing', () => {
  const data = fixture();
  assert.equal(assess({ ...data, bundle: null as unknown as MobilityBundle }).status, 'invalid');
  assert.equal(assess({ ...data, context: null as unknown as typeof data.context }).status, 'invalid');
  assert.equal(assess(data, null as unknown as string[]).status, 'invalid');
});
test('policy revision floors must be explicit own values and exact safe integers', () => {
  const data = fixture();
  addPolicy(data.bundle, data.context);
  data.bundle.policyRequirements[0].id = 'toString';
  data.context.policies[0].requirementId = 'toString';
  data.context.policies[0].bundleDigest = bundleDigest(data.bundle);
  data.context.minimumPolicyRevisions = {};
  const result = assess(data);
  assert.equal(result.status, 'unknown');
  assert.deepEqual(result.reasons.map(reason => reason.code), ['untrusted-policy-revision']);
  data.context.policies[0].revision = Number.MAX_SAFE_INTEGER + 1;
  assert.equal(assess(data).status, 'invalid');
});
