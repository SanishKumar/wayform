import assert from 'node:assert/strict';
import { assessPath } from '../packages/mobility-contract/src/assess';
import type { EvaluationContext, MobilityBundle, MobilityProfile } from '../packages/mobility-contract/src/types';
import { addEntrance, addObservation, addPolicy, fixture } from '../tests/fixtures';
import { policyInputs } from './policy-cases';

function evidenceInputs(bundle: MobilityBundle, profile: MobilityProfile, context: EvaluationContext) {
  return {
    records: bundle.evidence.map(item => ({ id: item.id, source: item.source, actorId: item.actorId, method: item.method, observedAtMs: Date.parse(item.observedAt), validUntilMs: item.validUntil === null ? null : Date.parse(item.validUntil), review: { status: item.review.status, reviewerId: item.review.reviewerId, reviewedAtMs: item.review.reviewedAt === null ? null : Date.parse(item.review.reviewedAt) } })),
    policy: { acceptedSources: profile.evidencePolicy.acceptedSources, requireReviewed: profile.evidencePolicy.requireReviewed, maxAgeSeconds: profile.evidencePolicy.maxAgeSeconds },
    evaluatedAtMs: Date.parse(context.evaluatedAt),
  };
}

export function buildEdgeCases() {
  type Data = ReturnType<typeof fixture>;
  const legacy = (accessible: boolean) => ({ accessible, meaning: 'author-assertion' as const, sourceCommit: 'a'.repeat(40), packageHash: 'b'.repeat(64), sourceEdgeId: 'original' });
  const cases: { id: string; status: string; change(data: Data): void }[] = [
    { id: 'measured', status: 'suitable', change() {} },
    { id: 'missing-width', status: 'unknown', change(data) { data.bundle.observations = []; } },
    { id: 'narrow', status: 'unsuitable', change(data) { data.bundle.observations[0].value = 0.6; } },
    { id: 'restricted-and-missing', status: 'unsuitable', change(data) { data.bundle.edges[0].access = 'restricted'; data.bundle.observations = []; } },
    { id: 'restricted-permitted', status: 'suitable', change(data) { data.bundle.edges[0].access = 'restricted'; data.profile.allowRestricted = true; } },
    { id: 'legacy-excluded', status: 'unsuitable', change(data) { data.bundle.edges[0].legacy = legacy(false); } },
    { id: 'legacy-exclusion-disabled', status: 'suitable', change(data) { data.bundle.edges[0].legacy = legacy(false); data.profile.respectLegacyExclusions = false; } },
    { id: 'legacy-true-without-measurement', status: 'unknown', change(data) { data.bundle.edges[0].legacy = legacy(true); data.bundle.observations = []; } },
    { id: 'stairs-and-missing-count', status: 'unsuitable', change(data) { data.bundle.edges[0].kind = 'stairs'; data.profile.hard.maxStepCount = 0; } },
    { id: 'stairs-declared-zero-count', status: 'unsuitable', change(data) { data.bundle.edges[0].kind = 'stairs'; data.profile.hard.maxStepCount = 0; addObservation(data.bundle, 'stepCount', 0, 0); } },
    { id: 'door-missing-operation', status: 'unknown', change(data) { data.bundle.edges[0].kind = 'door'; data.profile.hard.allowedDoorOperations = ['automatic']; } },
    { id: 'walkway-ignores-door-requirement', status: 'suitable', change(data) { data.profile.hard.allowedDoorOperations = ['automatic']; } },
    { id: 'closed-and-missing-width', status: 'unsuitable', change(data) { data.bundle.observations = []; addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; } },
    { id: 'expired-closure', status: 'unknown', change(data) { addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; data.context.evaluatedAt = data.context.policies[0].validUntil; } },
    { id: 'untrusted-policy', status: 'unknown', change(data) { addPolicy(data.bundle, data.context); data.context.trustedIssuerIds = []; } },
    { id: 'narrow-with-unknown-slope', status: 'unsuitable', change(data) { data.bundle.observations[0].value = 0.6; data.profile.hard.maxUphillPercent = 5; } },
    { id: 'all-physical-requirements', status: 'suitable', change(data) {
      data.bundle.edges[0].kind = 'door'; Object.assign(data.profile.hard, { maxUphillPercent: 5, maxDownhillPercent: 6, maxCrossSlopePercent: 2, maxStepCount: 0, allowedSurfaces: ['paved'], allowedDoorOperations: ['automatic'] });
      addObservation(data.bundle, 'slopePercent', -2, 0.1); addObservation(data.bundle, 'crossSlopePercent', 1, 0.1); addObservation(data.bundle, 'stepCount', 0, 0); addObservation(data.bundle, 'surface', 'paved', null); addObservation(data.bundle, 'doorOperation', 'automatic', null);
    } },
    { id: 'entrance', status: 'suitable', change(data) { addEntrance(data.bundle); } },
    { id: 'entrance-bad-angle', status: 'unknown', change(data) { addEntrance(data.bundle); data.bundle.transforms[0].rotationDegrees = 0; } },
    { id: 'entrance-overflow', status: 'unknown', change(data) { addEntrance(data.bundle); data.bundle.transforms[0].rotationDegrees = Number.MAX_VALUE; } },
    { id: 'entrance-large-uncertainty', status: 'unknown', change(data) { addEntrance(data.bundle); data.bundle.transforms[0].uncertaintyMeters = 0.3; } },
    { id: 'entrance-wrong-endpoint', status: 'unknown', change(data) { addEntrance(data.bundle); data.bundle.nodes[1].position[2] = 1; } },
    { id: 'entrance-reverse', status: 'suitable', change(data) { addEntrance(data.bundle); data.bundle.edges[0].from = 'b'; data.bundle.edges[0].to = 'a'; } },
    { id: 'entrance-y-down', status: 'suitable', change(data) { addEntrance(data.bundle); data.bundle.frames[0] = { id: 'plan', kind: 'local', axes: 'x-right-y-down-z-up' }; data.bundle.transforms[0].controls[1] = { local: [0, 10, 0], enu: [110, 200, 0] }; } },
    { id: 'entrance-stale-registration', status: 'unknown', change(data) {
      addEntrance(data.bundle); data.bundle.evidence.push({ ...data.bundle.evidence[0], id: 'registration-survey', validUntil: '2026-10-25T11:00:00.000Z' }); data.bundle.transforms[0].evidenceId = 'registration-survey';
    } },
    { id: 'entrance-closed-and-misaligned', status: 'unsuitable', change(data) { addEntrance(data.bundle); data.bundle.transforms[0].rotationDegrees = 0; addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; } },
  ];
  return cases.map(item => {
    const { bundle, profile, context } = (() => { const data = fixture(); item.change(data); return data; })();
    const edge = bundle.edges[0], from = bundle.nodes.find(node => node.id === edge.from)!, to = bundle.nodes.find(node => node.id === edge.to)!;
    const assessment = assessPath(bundle, profile, context, edge.from, edge.to, [edge.id]);
    assert.equal(assessment.status, item.status, item.id);
    const transform = bundle.transforms.find(transform => transform.id === edge.transformId);
    const frame = transform && bundle.frames.find(frame => frame.id === transform.localFrameId)!;
    const registration = transform ? {
      localYDown: frame?.kind === 'local' && frame.axes === 'x-right-y-down-z-up', rotationDegrees: transform.rotationDegrees, translationMeters: transform.translationMeters, uncertaintyMeters: transform.uncertaintyMeters, evidenceId: transform.evidenceId, controls: transform.controls,
      localEndpoint: (from.frameId === transform.localFrameId ? from : to).position, enuEndpoint: (from.frameId === transform.enuFrameId ? from : to).position,
    } : null;
    return {
      id: item.id, synthetic: true, policyInput: policyInputs(bundle, context), evidenceInput: evidenceInputs(bundle, profile, context),
      facts: { id: edge.id, kind: edge.kind, restricted: edge.access === 'restricted', legacyAccessible: edge.legacy?.accessible ?? null, observations: bundle.observations.filter(observation => observation.edgeId === edge.id).map(({ edgeId: _, ...observation }) => observation), crossesFrame: from.frameId !== to.frameId, registration },
      requirements: { allowRestricted: profile.allowRestricted, respectLegacyExclusions: profile.respectLegacyExclusions, hard: profile.hard, maxTransformErrorMeters: profile.evidencePolicy.maxTransformErrorMeters },
      expected: { status: assessment.status, reasons: assessment.reasons, evidenceIds: assessment.evidenceIds },
    };
  });
}
