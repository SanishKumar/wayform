import { CONTRACT_VERSION } from './types';

const str = { type: 'string', minLength: 1 };
const number = { type: 'number' };
const nonnegative = { type: 'number', minimum: 0 };
const integer = { type: 'integer', minimum: 0, maximum: Number.MAX_SAFE_INTEGER };
const nullable = (schema: object) => ({ anyOf: [schema, { type: 'null' }] });
const array = (items: object, minItems = 0) => ({ type: 'array', items, minItems });
const strings = { ...array(str, 1), uniqueItems: true };
const enumeration = (...values: string[]) => ({ type: 'string', enum: values });
const object = (properties: Record<string, object>) => ({ type: 'object', properties, required: Object.keys(properties), additionalProperties: false });
const point = { type: 'array', items: number, minItems: 3, maxItems: 3 };
const time = { type: 'string', format: 'utc-timestamp' };
const kind = enumeration('walkway', 'door', 'ramp', 'stairs', 'lift', 'crossing', 'entrance');
const source = enumeration('field-survey', 'operator', 'imported-map', 'inference');
const evidence = object({
  id: str, source, actorId: str, method: str, observedAt: time, validUntil: nullable(time),
  review: object({ status: enumeration('reviewed', 'pending', 'rejected'), reviewerId: nullable(str), reviewedAt: nullable(time) }),
});
const observation = object({
  id: str, edgeId: str, attribute: enumeration('clearWidthMeters', 'slopePercent', 'crossSlopePercent', 'stepCount', 'surface', 'doorOperation'),
  value: { anyOf: [number, str, { type: 'null' }] }, uncertainty: nullable(nonnegative), unknownReason: nullable(str), evidenceId: str,
});
export const bundleSchema = {
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  $id: 'urn:wayform:mobility-bundle:0.1.0-draft.1',
  ...object({
    schemaVersion: { const: CONTRACT_VERSION }, id: str, revision: str,
    frames: array({ oneOf: [
      object({ id: str, kind: { const: 'local' }, axes: enumeration('x-right-y-down-z-up', 'x-right-y-up-z-up') }),
      object({ id: str, kind: { const: 'enu' }, datum: { const: 'WGS84' }, anchor: object({ longitude: { type: 'number', minimum: -180, maximum: 180 }, latitude: { type: 'number', minimum: -90, maximum: 90 }, heightMeters: number }) }),
    ] }, 1),
    nodes: array(object({ id: str, frameId: str, position: point, floorId: nullable(str) }), 1),
    edges: array(object({ id: str, from: str, to: str, lengthMeters: { type: 'number', exclusiveMinimum: 0 }, kind, access: enumeration('public', 'restricted'), transformId: nullable(str), legacy: nullable(object({ accessible: { type: 'boolean' }, meaning: { const: 'author-assertion' }, sourceCommit: { type: 'string', pattern: '^[a-f0-9]{40}$' }, packageHash: { type: 'string', pattern: '^[a-f0-9]{64}$' }, sourceEdgeId: str })) })),
    evidence: array(evidence), observations: array(observation),
    transforms: array(object({ id: str, localFrameId: str, enuFrameId: str, rotationDegrees: number, translationMeters: point, uncertaintyMeters: nonnegative, evidenceId: str, controls: array(object({ local: point, enu: point }), 2) })),
    policyRequirements: array(object({ id: str, edgeIds: strings })),
  }),
};
export const profileSchema = {
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  $id: 'urn:wayform:mobility-profile:0.1.0-draft.1',
  ...object({
    id: str, allowRestricted: { type: 'boolean' }, respectLegacyExclusions: { type: 'boolean' },
    hard: object({ minClearWidthMeters: nullable(nonnegative), maxUphillPercent: nullable(nonnegative), maxDownhillPercent: nullable(nonnegative), maxCrossSlopePercent: nullable(nonnegative), maxStepCount: nullable(integer), allowedSurfaces: nullable(strings), allowedDoorOperations: nullable(strings) }),
    preferences: object({ avoidKinds: { ...array(kind), uniqueItems: true }, penaltyMeters: nonnegative }),
    evidencePolicy: object({ acceptedSources: { ...array(source, 1), uniqueItems: true }, requireReviewed: { type: 'boolean' }, maxAgeSeconds: nonnegative, maxTransformErrorMeters: nonnegative }),
  }),
};
export const contextSchema = {
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  $id: 'urn:wayform:evaluation-context:0.1.0-draft.1',
  ...object({
    evaluatedAt: time,
    policies: array(object({ requirementId: str, bundleId: str, bundleRevision: str, bundleDigest: { type: 'string', pattern: '^[a-f0-9]{64}$' }, revision: integer, issuerId: str, validFrom: time, validUntil: time, closedEdgeIds: { ...array(str), uniqueItems: true } })),
    trustedIssuerIds: { ...array(str), uniqueItems: true },
    minimumPolicyRevisions: { type: 'object', propertyNames: str, additionalProperties: integer },
  }),
};
