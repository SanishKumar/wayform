import Ajv2020 from 'ajv/dist/2020.js';
import { bundleSchema, contextSchema, profileSchema } from './schema';
import type { EvaluationContext, MobilityBundle, MobilityProfile } from './types';

const ajv = new Ajv2020({ allErrors: true, strict: true });
ajv.addFormat('utc-timestamp', { type: 'string', validate: (value: string) => {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(value)) return false;
  const parsed = new Date(value);
  return Number.isFinite(parsed.getTime()) && parsed.toISOString() === value;
} });
const bundleShape = ajv.compile<MobilityBundle>(bundleSchema);
const profileShape = ajv.compile<MobilityProfile>(profileSchema);
const contextShape = ajv.compile<EvaluationContext>(contextSchema);
const errors = (validator: typeof bundleShape) => (validator.errors ?? []).map(error => `${error.instancePath || '/'}: ${error.message}`);
export function validateBundle(value: unknown): string[] {
  if (!bundleShape(value)) return errors(bundleShape);
  const issues: string[] = [];
  for (const key of ['frames', 'nodes', 'edges', 'evidence', 'observations', 'transforms', 'policyRequirements'] as const) {
    const ids = value[key].map(item => item.id);
    if (new Set(ids).size !== ids.length) issues.push(`${key}: duplicate-id`);
  }
  const frames = new Map(value.frames.map(item => [item.id, item]));
  const nodes = new Map(value.nodes.map(item => [item.id, item]));
  const edges = new Map(value.edges.map(item => [item.id, item]));
  const evidence = new Map(value.evidence.map(item => [item.id, item]));
  const transforms = new Map(value.transforms.map(item => [item.id, item]));
  for (const node of value.nodes) if (!frames.has(node.frameId)) issues.push(`${node.id}: unknown-frame`);
  for (const edge of value.edges) {
    const from = nodes.get(edge.from), to = nodes.get(edge.to);
    if (!from || !to || from.id === to.id) { issues.push(`${edge.id}: invalid-endpoints`); continue; }
    if (from.frameId === to.frameId) {
      if (edge.transformId !== null) issues.push(`${edge.id}: unnecessary-transform`);
    } else {
      const transform = transforms.get(edge.transformId ?? '');
      const pair = new Set([from.frameId, to.frameId]);
      if (edge.kind !== 'entrance' || !transform || !pair.has(transform.localFrameId) || !pair.has(transform.enuFrameId)) issues.push(`${edge.id}: missing-or-wrong-entrance-transform`);
    }
  }
  for (const item of value.evidence) {
    if (item.validUntil !== null && Date.parse(item.validUntil) <= Date.parse(item.observedAt)) issues.push(`${item.id}: invalid-evidence-interval`);
    if (item.review.status === 'reviewed' && (!item.review.reviewerId || !item.review.reviewedAt)) issues.push(`${item.id}: missing-review-provenance`);
    if (item.review.reviewedAt !== null && Date.parse(item.review.reviewedAt) < Date.parse(item.observedAt)) issues.push(`${item.id}: review-before-observation`);
  }
  for (const observation of value.observations) {
    if (!edges.has(observation.edgeId) || !evidence.has(observation.evidenceId)) issues.push(`${observation.id}: missing-reference`);
    if (observation.value === null) {
      if (observation.unknownReason === null || observation.uncertainty !== null) issues.push(`${observation.id}: invalid-unknown`);
      continue;
    }
    if (observation.unknownReason !== null) issues.push(`${observation.id}: unexpected-unknown-reason`);
    const categorical = ['surface', 'doorOperation'].includes(observation.attribute);
    if (categorical) {
      if (typeof observation.value !== 'string' || observation.uncertainty !== null) issues.push(`${observation.id}: invalid-category`);
    } else if (typeof observation.value !== 'number') issues.push(`${observation.id}: invalid-measurement`);
    else {
      if (observation.attribute !== 'slopePercent' && observation.value < 0) issues.push(`${observation.id}: negative-measurement`);
      if (observation.attribute === 'stepCount' && (!Number.isSafeInteger(observation.value) || (observation.uncertainty !== null && !Number.isSafeInteger(observation.uncertainty)))) issues.push(`${observation.id}: nonintegral-step-count`);
    }
  }
  for (const transform of value.transforms) {
    if (frames.get(transform.localFrameId)?.kind !== 'local' || frames.get(transform.enuFrameId)?.kind !== 'enu' || !evidence.has(transform.evidenceId)) issues.push(`${transform.id}: invalid-transform-references`);
    const first = transform.controls[0];
    if (!transform.controls.some(control => Math.hypot(control.local[0] - first.local[0], control.local[1] - first.local[1]) > 0.001 && Math.hypot(control.enu[0] - first.enu[0], control.enu[1] - first.enu[1]) > 0.001)) issues.push(`${transform.id}: degenerate-controls`);
  }
  for (const requirement of value.policyRequirements) for (const id of requirement.edgeIds) if (!edges.has(id)) issues.push(`${requirement.id}: unknown-edge`);
  return issues.sort();
}
export function validateProfile(value: unknown): string[] {
  return profileShape(value) ? [] : (profileShape.errors ?? []).map(error => `${error.instancePath}: ${error.message}`);
}
export function validateContext(value: unknown, bundle: MobilityBundle): string[] {
  if (!contextShape(value)) return (contextShape.errors ?? []).map(error => `${error.instancePath}: ${error.message}`);
  const issues: string[] = [];
  const requirements = new Map(bundle.policyRequirements.map(item => [item.id, item]));
  const seen = new Set<string>();
  for (const policy of value.policies) {
    if (seen.has(policy.requirementId)) issues.push('duplicate-policy');
    seen.add(policy.requirementId);
    const requirement = requirements.get(policy.requirementId);
    if (!requirement || policy.closedEdgeIds.some(id => !requirement.edgeIds.includes(id))) issues.push('invalid-policy-target');
    if (Date.parse(policy.validUntil) <= Date.parse(policy.validFrom)) issues.push('invalid-policy-interval');
  }
  for (const id of Object.keys(value.minimumPolicyRevisions)) if (!requirements.has(id)) issues.push('unknown-policy-revision-floor');
  return issues.sort();
}
