import { CONTRACT_VERSION, type EvaluationContext, type MobilityBundle, type MobilityProfile, type Observation } from '../packages/mobility-contract/src/types';
import { bundleDigest } from '../packages/mobility-contract/src/identity';

export function fixture(): { bundle: MobilityBundle; profile: MobilityProfile; context: EvaluationContext } {
  return {
    bundle: {
      schemaVersion: CONTRACT_VERSION, id: 'synthetic-venue', revision: 'r1',
      frames: [{ id: 'plan', kind: 'local', axes: 'x-right-y-up-z-up' }],
      nodes: [
        { id: 'a', frameId: 'plan', position: [0, 0, 0], floorId: 'g' },
        { id: 'b', frameId: 'plan', position: [10, 0, 0], floorId: 'g' },
        { id: 'c', frameId: 'plan', position: [10, 10, 0], floorId: 'g' },
      ],
      edges: [{ id: 'ab', from: 'a', to: 'b', lengthMeters: 10, kind: 'walkway', access: 'public', transformId: null, legacy: null }],
      evidence: [{ id: 'survey', source: 'field-survey', actorId: 'fictional-surveyor', method: 'synthetic-test', observedAt: '2026-10-25T10:00:00.000Z', validUntil: null, review: { status: 'reviewed', reviewerId: 'fictional-reviewer', reviewedAt: '2026-10-25T10:15:00.000Z' } }],
      observations: [{ id: 'width', edgeId: 'ab', attribute: 'clearWidthMeters', value: 1, uncertainty: 0.02, unknownReason: null, evidenceId: 'survey' }],
      transforms: [], policyRequirements: [],
    },
    profile: {
      id: 'synthetic-profile', allowRestricted: false, respectLegacyExclusions: true,
      hard: { minClearWidthMeters: 0.8, maxUphillPercent: null, maxDownhillPercent: null, maxCrossSlopePercent: null, maxStepCount: null, allowedSurfaces: null, allowedDoorOperations: null },
      preferences: { avoidKinds: [], penaltyMeters: 0 },
      evidencePolicy: { acceptedSources: ['field-survey', 'operator'], requireReviewed: true, maxAgeSeconds: 86400, maxTransformErrorMeters: 0.25 },
    },
    context: { evaluatedAt: '2026-10-25T12:00:00.000Z', policies: [], trustedIssuerIds: ['venue-operator'], minimumPolicyRevisions: {} },
  };
}
export function addObservation(bundle: MobilityBundle, attribute: Observation['attribute'], value: Observation['value'], uncertainty: number | null = 0, edgeId = 'ab') {
  bundle.observations.push({ id: `${edgeId}:${attribute}:${bundle.observations.length}`, edgeId, attribute, value, uncertainty, unknownReason: value === null ? 'not-surveyed' : null, evidenceId: 'survey' });
}
export function addPolicy(bundle: MobilityBundle, context: EvaluationContext) {
  bundle.policyRequirements.push({ id: 'operations', edgeIds: ['ab'] });
  context.minimumPolicyRevisions.operations = 3;
  context.policies.push({ requirementId: 'operations', bundleId: bundle.id, bundleRevision: bundle.revision, bundleDigest: bundleDigest(bundle), revision: 3, issuerId: 'venue-operator', validFrom: '2026-10-25T11:00:00.000Z', validUntil: '2026-10-25T13:00:00.000Z', closedEdgeIds: [] });
}
export function addEntrance(bundle: MobilityBundle) {
  bundle.frames.push({ id: 'outside', kind: 'enu', datum: 'WGS84', anchor: { longitude: 77, latitude: 28, heightMeters: 200 } });
  bundle.nodes[1].frameId = 'outside';
  bundle.nodes[1].position = [100, 200, 0];
  bundle.edges[0].kind = 'entrance';
  bundle.edges[0].transformId = 'entrance';
  bundle.transforms.push({ id: 'entrance', localFrameId: 'plan', enuFrameId: 'outside', rotationDegrees: 90, translationMeters: [100, 200, 0], uncertaintyMeters: 0.05, evidenceId: 'survey', controls: [{ local: [0, 0, 0], enu: [100, 200, 0] }, { local: [10, 0, 0], enu: [100, 210, 0] }] });
}
