export const CONTRACT_VERSION = '0.1.0-draft.1' as const;
export type Point3 = [number, number, number];
export type Attribute = 'clearWidthMeters' | 'slopePercent' | 'crossSlopePercent' | 'stepCount' | 'surface' | 'doorOperation';
export type EvidenceSource = 'field-survey' | 'operator' | 'imported-map' | 'inference';
export type EdgeKind = 'walkway' | 'door' | 'ramp' | 'stairs' | 'lift' | 'crossing' | 'entrance';
export interface Evidence {
  id: string;
  source: EvidenceSource;
  actorId: string;
  method: string;
  observedAt: string;
  validUntil: string | null;
  review: { status: 'reviewed' | 'pending' | 'rejected'; reviewerId: string | null; reviewedAt: string | null };
}
export interface Observation {
  id: string;
  edgeId: string;
  attribute: Attribute;
  value: number | string | null;
  /** Absolute error bound in the attribute's unit, not a probability. */
  uncertainty: number | null;
  unknownReason: string | null;
  evidenceId: string;
}
export type Frame =
  | { id: string; kind: 'local'; axes: 'x-right-y-down-z-up' | 'x-right-y-up-z-up' }
  | { id: string; kind: 'enu'; anchor: { longitude: number; latitude: number; heightMeters: number }; datum: 'WGS84' };
export interface MobilityNode { id: string; frameId: string; position: Point3; floorId: string | null }
export interface LegacyAssertion {
  accessible: boolean;
  meaning: 'author-assertion';
  sourceCommit: string;
  packageHash: string;
  sourceEdgeId: string;
}
export interface DirectedEdge {
  id: string;
  from: string;
  to: string;
  lengthMeters: number;
  kind: EdgeKind;
  access: 'public' | 'restricted';
  transformId: string | null;
  legacy: LegacyAssertion | null;
}
export interface EntranceTransform {
  id: string;
  localFrameId: string;
  enuFrameId: string;
  /** Counterclockwise rotation after converting local plan Y to up. */
  rotationDegrees: number;
  translationMeters: Point3;
  uncertaintyMeters: number;
  evidenceId: string;
  controls: { local: Point3; enu: Point3 }[];
}
export interface MobilityBundle {
  schemaVersion: typeof CONTRACT_VERSION;
  id: string;
  revision: string;
  frames: Frame[];
  nodes: MobilityNode[];
  edges: DirectedEdge[];
  evidence: Evidence[];
  observations: Observation[];
  transforms: EntranceTransform[];
  policyRequirements: { id: string; edgeIds: string[] }[];
}
export interface MobilityProfile {
  id: string;
  allowRestricted: boolean;
  respectLegacyExclusions: boolean;
  hard: {
    minClearWidthMeters: number | null;
    maxUphillPercent: number | null;
    maxDownhillPercent: number | null;
    maxCrossSlopePercent: number | null;
    maxStepCount: number | null;
    allowedSurfaces: string[] | null;
    allowedDoorOperations: string[] | null;
  };
  preferences: { avoidKinds: EdgeKind[]; penaltyMeters: number };
  evidencePolicy: { acceptedSources: EvidenceSource[]; requireReviewed: boolean; maxAgeSeconds: number; maxTransformErrorMeters: number };
}
export interface OperationalPolicy {
  requirementId: string;
  bundleId: string;
  bundleRevision: string;
  bundleDigest: string;
  revision: number;
  issuerId: string;
  validFrom: string;
  validUntil: string;
  closedEdgeIds: string[];
}
/** Policies must already have passed authenticated ingestion; this is not a signature verifier. */
export interface EvaluationContext {
  evaluatedAt: string;
  policies: OperationalPolicy[];
  trustedIssuerIds: string[];
  minimumPolicyRevisions: Record<string, number>;
}
export interface Reason { code: string; edgeId: string; attribute: Attribute | null; evidenceIds: string[] }
export interface Assessment {
  status: 'suitable' | 'unsuitable' | 'unknown' | 'invalid';
  reasons: Reason[];
  evidenceIds: string[];
}
export interface PathAssessment extends Assessment {
  receipt: {
    schemaVersion: typeof CONTRACT_VERSION;
    bundleId: string;
    bundleRevision: string;
    bundleDigest: string | null;
    profileDigest: string | null;
    contextDigest: string | null;
    profileId: string;
    evaluatedAt: string;
    edgeIds: string[];
    policyRevisions: { requirementId: string; revision: number }[];
    distanceMeters: number | null;
    preferenceCostMeters: number | null;
  };
}
