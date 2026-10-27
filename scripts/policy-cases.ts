import assert from 'node:assert/strict';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { bundleDigest } from '../packages/mobility-contract/src/identity';
import type { MobilityBundle, EvaluationContext } from '../packages/mobility-contract/src/types';
import { addPolicy, fixture } from '../tests/fixtures';

export function policyInputs(bundle: MobilityBundle, context: EvaluationContext) {
  return {
    bundle: { identity: { id: bundle.id, revision: bundle.revision, digest: bundleDigest(bundle) }, edgeIds: bundle.edges.map(edge => edge.id), requirements: bundle.policyRequirements },
    context: { evaluatedAtMs: Date.parse(context.evaluatedAt), trustedIssuerIds: context.trustedIssuerIds, minimumRevisions: Object.entries(context.minimumPolicyRevisions).map(([requirementId, revision]) => ({ requirementId, revision })),
      policies: context.policies.map(item => ({ requirementId: item.requirementId, bundle: { id: item.bundleId, revision: item.bundleRevision, digest: item.bundleDigest }, revision: item.revision, issuerId: item.issuerId, validFromMs: Date.parse(item.validFrom), validUntilMs: Date.parse(item.validUntil), closedEdgeIds: item.closedEdgeIds })),
    },
  };
}

export function buildPolicyCases() {
  type Data = ReturnType<typeof fixture>;
  const cases: { id: string; status: string; change(data: Data): void; edgeId?: string }[] = [
    { id: 'current-open', status: 'suitable', change() {} },
    { id: 'current-closed', status: 'unsuitable', change(data) { data.context.policies[0].closedEdgeIds = ['ab']; } },
    { id: 'missing', status: 'unknown', change(data) { data.context.policies = []; } },
    { id: 'not-required', status: 'suitable', change(data) { data.bundle.policyRequirements = []; data.context.policies = []; data.context.minimumPolicyRevisions = {}; } },
    { id: 'expired-open', status: 'unknown', change(data) { data.context.evaluatedAt = data.context.policies[0].validUntil; } },
    { id: 'expired-closed', status: 'unknown', change(data) { data.context.evaluatedAt = data.context.policies[0].validUntil; data.context.policies[0].closedEdgeIds = ['ab']; } },
    { id: 'not-yet-valid', status: 'unknown', change(data) { data.context.evaluatedAt = '2026-10-25T10:59:59.999Z'; } },
    { id: 'valid-from-boundary', status: 'suitable', change(data) { data.context.evaluatedAt = data.context.policies[0].validFrom; } },
    { id: 'just-before-expiry', status: 'suitable', change(data) { data.context.evaluatedAt = '2026-10-25T12:59:59.999Z'; } },
    { id: 'wrong-bundle-id', status: 'unknown', change(data) { data.context.policies[0].bundleId = 'other'; } },
    { id: 'wrong-bundle-revision', status: 'unknown', change(data) { data.context.policies[0].bundleRevision = 'other'; } },
    { id: 'changed-bundle-content', status: 'unknown', change(data) { data.bundle.edges[0].lengthMeters = 11; } },
    { id: 'untrusted-issuer', status: 'unknown', change(data) { data.context.trustedIssuerIds = []; } },
    { id: 'rollback', status: 'unknown', change(data) { data.context.policies[0].revision = 2; } },
    { id: 'missing-floor', status: 'unknown', change(data) { data.context.minimumPolicyRevisions = {}; } },
    { id: 'explicit-zero-floor', status: 'suitable', change(data) { data.context.minimumPolicyRevisions.operations = 0; data.context.policies[0].revision = 0; } },
    { id: 'maximum-safe-revision', status: 'suitable', change(data) { data.context.minimumPolicyRevisions.operations = Number.MAX_SAFE_INTEGER; data.context.policies[0].revision = Number.MAX_SAFE_INTEGER; } },
    { id: 'identity-before-trust-and-expiry', status: 'unknown', change(data) { data.context.policies[0].bundleRevision = 'other'; data.context.trustedIssuerIds = []; data.context.evaluatedAt = data.context.policies[0].validUntil; } },
    { id: 'trust-before-revision-and-expiry', status: 'unknown', change(data) { data.context.trustedIssuerIds = []; data.context.policies[0].revision = 2; data.context.evaluatedAt = data.context.policies[0].validUntil; } },
    { id: 'revision-before-expiry', status: 'unknown', change(data) { data.context.policies[0].revision = 2; data.context.evaluatedAt = data.context.policies[0].validUntil; } },
    { id: 'reverse-edge-unaffected', status: 'suitable', edgeId: 'ba', change(data) { data.bundle.edges.push({ ...data.bundle.edges[0], id: 'ba', from: 'b', to: 'a' }); data.context.policies[0].bundleDigest = bundleDigest(data.bundle); data.context.policies[0].closedEdgeIds = ['ab']; } },
    { id: 'closed-and-missing', status: 'unsuitable', change(data) { data.bundle.policyRequirements.push({ id: 'door', edgeIds: ['ab'] }); data.context.policies[0].bundleDigest = bundleDigest(data.bundle); data.context.policies[0].closedEdgeIds = ['ab']; } },
    { id: 'multiple-missing', status: 'unknown', change(data) { data.bundle.policyRequirements.push({ id: 'door', edgeIds: ['ab'] }); data.context.policies = []; } },
  ];
  return cases.map(item => {
    const data = fixture(); data.profile.hard.minClearWidthMeters = null; addPolicy(data.bundle, data.context); item.change(data);
    const edgeId = item.edgeId ?? 'ab', edge = data.bundle.edges.find(edge => edge.id === edgeId)!;
    const expected = assessPath(data.bundle, data.profile, data.context, edge.from, edge.to, [edgeId]);
    assert.equal(expected.status, item.status, item.id);
    assert(expected.reasons.every(reason => reason.attribute === null && reason.evidenceIds.length === 0), item.id);
    return { id: item.id, synthetic: true, ...policyInputs(data.bundle, data.context), edgeId, expected: { status: expected.status, codes: expected.reasons.map(reason => reason.code) } };
  });
}
