import { createHash } from 'node:crypto';
import type { EvaluationContext, MobilityBundle, MobilityProfile } from './types';

const compare = (a: string, b: string) => a < b ? -1 : a > b ? 1 : 0;
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === 'object') return Object.fromEntries(Object.entries(value).sort(([a], [b]) => compare(a, b)).map(([key, child]) => [key, canonical(child)]));
  return value;
}
const hash = (value: unknown) => createHash('sha256').update(JSON.stringify(canonical(value))).digest('hex');
const byId = <T extends { id: string }>(items: T[]) => [...items].sort((a, b) => compare(a.id, b.id));
/** Identity of the declared JSON inputs; no publisher authenticity or factual verification. */
export function bundleDigest(bundle: MobilityBundle) {
  return hash({ ...bundle,
      frames: byId(bundle.frames), nodes: byId(bundle.nodes), edges: byId(bundle.edges), evidence: byId(bundle.evidence), observations: byId(bundle.observations),
      transforms: byId(bundle.transforms).map(transform => ({ ...transform, controls: [...transform.controls].sort((a, b) => compare(JSON.stringify(canonical(a)), JSON.stringify(canonical(b)))) })),
      policyRequirements: byId(bundle.policyRequirements).map(item => ({ ...item, edgeIds: [...item.edgeIds].sort(compare) })),
  });
}
export function inputDigests(bundle: MobilityBundle, profile: MobilityProfile, context: EvaluationContext) {
  return {
    bundleDigest: bundleDigest(bundle),
    profileDigest: hash({ ...profile,
      hard: { ...profile.hard, allowedSurfaces: profile.hard.allowedSurfaces && [...profile.hard.allowedSurfaces].sort(compare), allowedDoorOperations: profile.hard.allowedDoorOperations && [...profile.hard.allowedDoorOperations].sort(compare) },
      preferences: { ...profile.preferences, avoidKinds: [...profile.preferences.avoidKinds].sort(compare) },
      evidencePolicy: { ...profile.evidencePolicy, acceptedSources: [...profile.evidencePolicy.acceptedSources].sort(compare) },
    }),
    contextDigest: hash({ ...context, trustedIssuerIds: [...context.trustedIssuerIds].sort(compare), policies: [...context.policies].sort((a, b) => compare(a.requirementId, b.requirementId)).map(item => ({ ...item, closedEdgeIds: [...item.closedEdgeIds].sort(compare) })) }),
  };
}
