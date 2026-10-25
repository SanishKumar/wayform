import { CONTRACT_VERSION, type Assessment, type Attribute, type DirectedEdge, type EntranceTransform, type EvaluationContext, type Evidence, type MobilityBundle, type MobilityProfile, type PathAssessment, type Point3, type Reason } from './types';
import { validateBundle, validateContext, validateProfile } from './validate';
import { inputDigests } from './identity';

const sorted = (values: string[]) => [...new Set(values)].sort();
// Only absorbs IEEE-754 arithmetic noise at an inclusive threshold. Survey error
// remains in the interval and is never replaced by this numerical tolerance.
const below = (left: number, right: number) => left < right &&
  (!Number.isFinite(left) || !Number.isFinite(right) || right - left > 8 * Number.EPSILON * Math.max(1, Math.abs(left), Math.abs(right)));
function finish(statuses: Assessment['status'][], reasons: Reason[], evidenceIds: string[]): Assessment {
  reasons.sort((a, b) => {
    const left = `${a.edgeId}/${a.attribute ?? ''}/${a.code}`, right = `${b.edgeId}/${b.attribute ?? ''}/${b.code}`;
    return left < right ? -1 : left > right ? 1 : 0;
  });
  const status = (['invalid', 'unsuitable', 'unknown'] as const).find(value => statuses.includes(value)) ?? 'suitable';
  return { status, reasons, evidenceIds: sorted(evidenceIds) };
}
function evidenceIssue(evidence: Evidence, profile: MobilityProfile, now: number): string | null {
  if (!profile.evidencePolicy.acceptedSources.includes(evidence.source)) return 'unaccepted-evidence-source';
  if (evidence.review.status === 'rejected' || (profile.evidencePolicy.requireReviewed && evidence.review.status !== 'reviewed')) return 'unreviewed-evidence';
  if (Date.parse(evidence.observedAt) > now || (evidence.review.reviewedAt !== null && Date.parse(evidence.review.reviewedAt) > now)) return 'future-evidence';
  if ((now - Date.parse(evidence.observedAt)) / 1000 > profile.evidencePolicy.maxAgeSeconds || (evidence.validUntil !== null && now >= Date.parse(evidence.validUntil))) return 'stale-evidence';
  return null;
}
/** Rigid metric transform only. Converting WGS84 positions to ENU is outside this draft evaluator. */
export function localToEnu(point: Point3, transform: EntranceTransform, yDown: boolean): Point3 {
  const angle = transform.rotationDegrees * Math.PI / 180;
  const x = point[0], y = point[1] * (yDown ? -1 : 1);
  return [Math.cos(angle) * x - Math.sin(angle) * y + transform.translationMeters[0], Math.sin(angle) * x + Math.cos(angle) * y + transform.translationMeters[1], point[2] + transform.translationMeters[2]];
}
const distance = (left: Point3, right: Point3) => Math.hypot(...left.map((value, index) => value - right[index]));

function assessEdge(bundle: MobilityBundle, edge: DirectedEdge, profile: MobilityProfile, context: EvaluationContext, digest: string): Assessment {
  const statuses: Assessment['status'][] = [], reasons: Reason[] = [], used: string[] = [];
  const note = (status: Assessment['status'], code: string, attribute: Attribute | null = null, ids: string[] = []) => {
    statuses.push(status); reasons.push({ code, edgeId: edge.id, attribute, evidenceIds: sorted(ids) }); used.push(...ids);
  };
  const now = Date.parse(context.evaluatedAt);
  const evidence = new Map(bundle.evidence.map(item => [item.id, item]));
  if (edge.access === 'restricted' && !profile.allowRestricted) note('unsuitable', 'restricted-access');
  if (profile.respectLegacyExclusions && edge.legacy?.accessible === false) note('unsuitable', 'legacy-declared-inaccessible');
  if (edge.kind === 'stairs' && profile.hard.maxStepCount === 0) note('unsuitable', 'stairs-excluded', 'stepCount');
  for (const requirement of bundle.policyRequirements.filter(item => item.edgeIds.includes(edge.id))) {
    const policy = context.policies.find(item => item.requirementId === requirement.id);
    const floor = Object.hasOwn(context.minimumPolicyRevisions, requirement.id) ? context.minimumPolicyRevisions[requirement.id] : undefined;
    if (!policy) { note('unknown', 'missing-operational-policy'); continue; }
    if (policy.bundleId !== bundle.id || policy.bundleRevision !== bundle.revision || policy.bundleDigest !== digest) { note('unknown', 'policy-bundle-mismatch'); continue; }
    if (!context.trustedIssuerIds.includes(policy.issuerId)) { note('unknown', 'untrusted-policy-issuer'); continue; }
    if (floor === undefined || policy.revision < floor) { note('unknown', 'untrusted-policy-revision'); continue; }
    if (now < Date.parse(policy.validFrom) || now >= Date.parse(policy.validUntil)) { note('unknown', 'policy-not-current'); continue; }
    if (policy.closedEdgeIds.includes(edge.id)) note('unsuitable', 'operationally-closed');
  }
  if (edge.transformId !== null) {
    const transform = bundle.transforms.find(item => item.id === edge.transformId)!;
    const source = evidence.get(transform.evidenceId)!;
    used.push(source.id);
    const issue = evidenceIssue(source, profile, now);
    if (issue) note('unknown', `transform-${issue}`, null, [source.id]);
    const frame = bundle.frames.find(item => item.id === transform.localFrameId)!;
    const yDown = frame.kind === 'local' && frame.axes === 'x-right-y-down-z-up';
    const residual = Math.max(...transform.controls.map(control => distance(localToEnu(control.local, transform, yDown), control.enu)));
    const endpoints = [bundle.nodes.find(item => item.id === edge.from)!, bundle.nodes.find(item => item.id === edge.to)!];
    const local = endpoints.find(item => item.frameId === transform.localFrameId)!;
    const enu = endpoints.find(item => item.frameId === transform.enuFrameId)!;
    const joinError = distance(localToEnu(local.position, transform, yDown), enu.position);
    if (Math.max(residual, joinError) + transform.uncertaintyMeters > profile.evidencePolicy.maxTransformErrorMeters) note('unknown', 'transform-error-exceeds-budget', null, [source.id]);
  }
  const constraints: { attribute: Attribute; min?: number; max?: number; allowed?: string[] }[] = [];
  const hard = profile.hard;
  if (hard.minClearWidthMeters !== null) constraints.push({ attribute: 'clearWidthMeters', min: hard.minClearWidthMeters });
  if (hard.maxUphillPercent !== null || hard.maxDownhillPercent !== null) constraints.push({ attribute: 'slopePercent', min: hard.maxDownhillPercent === null ? undefined : -hard.maxDownhillPercent, max: hard.maxUphillPercent ?? undefined });
  if (hard.maxCrossSlopePercent !== null) constraints.push({ attribute: 'crossSlopePercent', max: hard.maxCrossSlopePercent });
  if (hard.maxStepCount !== null) constraints.push({ attribute: 'stepCount', max: hard.maxStepCount });
  if (hard.allowedSurfaces !== null) constraints.push({ attribute: 'surface', allowed: hard.allowedSurfaces });
  // Door operation is only required for door edges; every traversed door must be modeled explicitly.
  if (hard.allowedDoorOperations !== null && edge.kind === 'door') constraints.push({ attribute: 'doorOperation', allowed: hard.allowedDoorOperations });
  for (const constraint of constraints) {
    const observations = bundle.observations.filter(item => item.edgeId === edge.id && item.attribute === constraint.attribute);
    const valid = observations.filter(item => !evidenceIssue(evidence.get(item.evidenceId)!, profile, now) && item.value !== null && (typeof item.value === 'string' || item.uncertainty !== null));
    if (!valid.length) {
      const codes = observations.length ? sorted(observations.map(item => evidenceIssue(evidence.get(item.evidenceId)!, profile, now) ?? (item.value === null ? 'unknown-measurement' : 'unknown-measurement-error'))) : ['missing-measurement'];
      for (const code of codes) note('unknown', code, constraint.attribute, observations.map(item => item.evidenceId));
      continue;
    }
    const ids = valid.map(item => item.evidenceId);
    used.push(...ids);
    if (constraint.allowed) {
      const values = sorted(valid.map(item => String(item.value)));
      if (values.length !== 1) note('unknown', 'contradictory-evidence', constraint.attribute, ids);
      else if (!constraint.allowed.includes(values[0])) note('unsuitable', 'hard-requirement-failed', constraint.attribute, ids);
    } else {
      const intervals = valid.map(item => [Number(item.value) - item.uncertainty!, Number(item.value) + item.uncertainty!] as const);
      // Disjoint measurements are a conflict; agreement never shrinks uncertainty by averaging.
      if (below(Math.min(...intervals.map(item => item[1])), Math.max(...intervals.map(item => item[0])))) { note('unknown', 'contradictory-evidence', constraint.attribute, ids); continue; }
      const low = Math.min(...intervals.map(item => item[0])), high = Math.max(...intervals.map(item => item[1]));
      const min = constraint.min ?? -Infinity, max = constraint.max ?? Infinity;
      if (below(high, min) || below(max, low)) note('unsuitable', 'hard-requirement-failed', constraint.attribute, ids);
      else if (below(low, min) || below(max, high)) note('unknown', 'measurement-straddles-limit', constraint.attribute, ids);
    }
  }
  return finish(statuses, reasons, used);
}

/** A contract oracle for a supplied path, not a graph search or a physical safety guarantee. */
export function assessPath(bundle: MobilityBundle, profile: MobilityProfile, context: EvaluationContext, startId: string, destinationId: string, edgeIds: string[]): PathAssessment {
  const label = (value: unknown, key: string) => value !== null && typeof value === 'object' && typeof (value as Record<string, unknown>)[key] === 'string' ? (value as Record<string, string>)[key] : '<invalid>';
  const validPath = Array.isArray(edgeIds) && edgeIds.every(id => typeof id === 'string' && id.length > 0);
  const receipt: PathAssessment['receipt'] = { schemaVersion: CONTRACT_VERSION, bundleId: label(bundle, 'id'), bundleRevision: label(bundle, 'revision'), bundleDigest: null, profileDigest: null, contextDigest: null, profileId: label(profile, 'id'), evaluatedAt: label(context, 'evaluatedAt'), edgeIds: validPath ? [...edgeIds] : [], policyRevisions: [], distanceMeters: null, preferenceCostMeters: null };
  const invalid = (code: string): PathAssessment => ({ ...finish(['invalid'], [{ code, edgeId: '', attribute: null, evidenceIds: [] }], []), receipt });
  if (!validPath || validateBundle(bundle).length || validateProfile(profile).length || validateContext(context, bundle).length) return invalid('invalid-contract');
  Object.assign(receipt, inputDigests(bundle, profile, context));
  const nodes = new Set(bundle.nodes.map(node => node.id));
  if (!nodes.has(startId) || !nodes.has(destinationId)) return invalid('unknown-endpoint');
  const edges = new Map(bundle.edges.map(edge => [edge.id, edge]));
  let current = startId, metres = 0, cost = 0;
  const assessments: Assessment[] = [];
  const usedRequirements = new Set<string>();
  for (const id of edgeIds) {
    const edge = edges.get(id);
    if (!edge || edge.from !== current) return invalid('disconnected-or-reversed-edge');
    current = edge.to; metres += edge.lengthMeters;
    cost += edge.lengthMeters + (profile.preferences.avoidKinds.includes(edge.kind) ? profile.preferences.penaltyMeters : 0);
    assessments.push(assessEdge(bundle, edge, profile, context, receipt.bundleDigest!));
    for (const requirement of bundle.policyRequirements) if (requirement.edgeIds.includes(id)) usedRequirements.add(requirement.id);
  }
  if (current !== destinationId) return invalid('destination-not-reached');
  if (!Number.isFinite(metres) || !Number.isFinite(cost)) return invalid('cost-overflow');
  receipt.distanceMeters = metres;
  receipt.preferenceCostMeters = cost;
  receipt.policyRevisions = context.policies.filter(item => usedRequirements.has(item.requirementId)).map(item => ({ requirementId: item.requirementId, revision: item.revision })).sort((a, b) => a.requirementId < b.requirementId ? -1 : a.requirementId > b.requirementId ? 1 : 0);
  return { ...finish(assessments.map(item => item.status), assessments.flatMap(item => item.reasons), assessments.flatMap(item => item.evidenceIds)), receipt };
}
