import assert from 'node:assert/strict';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { inputDigests } from '../packages/mobility-contract/src/identity';
import { validateBundle, validateContext, validateProfile } from '../packages/mobility-contract/src/validate';
import { addEntrance, addObservation, addPolicy, fixture } from '../tests/fixtures';

type Path = (string | number)[];
export type Edit = { op: 'set'; path: Path; value: unknown } | { op: 'remove'; path: Path };
export function applyEdits<T>(input: T, edits: Edit[]): T {
  const output = structuredClone(input);
  for (const edit of edits) {
    let parent: any = output;
    for (const key of edit.path.slice(0, -1)) parent = parent[key];
    const key = edit.path.at(-1)!;
    if (edit.op === 'remove') delete parent[key]; else parent[key] = structuredClone(edit.value);
  }
  return output;
}

export function buildLoaderCases() {
  const base = fixture();
  addEntrance(base.bundle);
  base.bundle.edges[0].legacy = { accessible: true, meaning: 'author-assertion', sourceCommit: 'a'.repeat(40), packageHash: 'b'.repeat(64), sourceEdgeId: 'original' };
  Object.assign(base.profile.hard, { maxUphillPercent: 5, maxDownhillPercent: 6, maxCrossSlopePercent: 2, maxStepCount: 0, allowedSurfaces: ['paved'], allowedDoorOperations: ['automatic'] });
  addObservation(base.bundle, 'slopePercent', -2, 0.1);
  addObservation(base.bundle, 'crossSlopePercent', 1, 0.1);
  addObservation(base.bundle, 'stepCount', 0, 0);
  addObservation(base.bundle, 'surface', 'paved', null);
  addPolicy(base.bundle, base.context);
  const definitions: { id: string; edits: Edit[]; valid: boolean }[] = [{ id: 'complete', edits: [], valid: true }];
  const set = (id: string, path: Path, value: unknown, valid = false) => definitions.push({ id, edits: [{ op: 'set', path, value }], valid });
  const walk = (value: unknown, path: Path) => {
    if (Array.isArray(value)) { value.forEach((child, index) => walk(child, [...path, index])); return; }
    if (value === null || typeof value !== 'object') return;
    if (path.length && path.at(-1) !== 'minimumPolicyRevisions') {
      for (const key of Object.keys(value)) definitions.push({ id: `missing:${[...path, key].join('/')}`, edits: [{ op: 'remove', path: [...path, key] }], valid: false });
      set(`extra:${path.join('/')}`, [...path, 'unexpected'], true);
    }
    for (const [key, child] of Object.entries(value)) walk(child, [...path, key]);
  };
  walk(base, []);
  for (const field of ['frames', 'nodes', 'edges', 'evidence', 'observations', 'transforms', 'policyRequirements'] as const) {
    set(`duplicate:${field}`, ['bundle', field], [...base.bundle[field], base.bundle[field][0]]);
  }
  const checks: [string, Path, unknown, boolean?][] = [
    ['wrong-version', ['bundle', 'schemaVersion'], '1.0.0'],
    ['empty-id', ['bundle', 'id'], ''],
    ['empty-nodes', ['bundle', 'nodes'], []],
    ['wrong-frame', ['bundle', 'nodes', 0, 'frameId'], 'missing'],
    ['short-point', ['bundle', 'nodes', 0, 'position'], [0, 0]],
    ['null-floor', ['bundle', 'nodes', 0, 'floorId'], null, true],
    ['empty-floor', ['bundle', 'nodes', 0, 'floorId'], ''],
    ['longitude-range', ['bundle', 'frames', 1, 'anchor', 'longitude'], 181],
    ['latitude-range', ['bundle', 'frames', 1, 'anchor', 'latitude'], -91],
    ['wrong-datum', ['bundle', 'frames', 1, 'datum'], 'other'],
    ['wrong-axes', ['bundle', 'frames', 0, 'axes'], 'other'],
    ['endpoint', ['bundle', 'edges', 0, 'to'], 'missing'],
    ['self-loop', ['bundle', 'edges', 0, 'to'], 'a'],
    ['zero-length', ['bundle', 'edges', 0, 'lengthMeters'], 0],
    ['negative-length', ['bundle', 'edges', 0, 'lengthMeters'], -1],
    ['unknown-kind', ['bundle', 'edges', 0, 'kind'], 'teleport'],
    ['bad-access', ['bundle', 'edges', 0, 'access'], true],
    ['cross-frame-kind', ['bundle', 'edges', 0, 'kind'], 'walkway'],
    ['cross-frame-no-transform', ['bundle', 'edges', 0, 'transformId'], null],
    ['cross-frame-wrong-transform', ['bundle', 'edges', 0, 'transformId'], 'missing'],
    ['legacy-null', ['bundle', 'edges', 0, 'legacy'], null, true],
    ['legacy-semantics', ['bundle', 'edges', 0, 'legacy', 'meaning'], 'surveyed'],
    ['legacy-commit', ['bundle', 'edges', 0, 'legacy', 'sourceCommit'], 'A'.repeat(40)],
    ['legacy-hash', ['bundle', 'edges', 0, 'legacy', 'packageHash'], 'b'.repeat(63)],
    ['review-provenance', ['bundle', 'evidence', 0, 'review', 'reviewerId'], null],
    ['review-before-observation', ['bundle', 'evidence', 0, 'review', 'reviewedAt'], '2026-10-24T00:00:00.000Z'],
    ['expiry-before-observation', ['bundle', 'evidence', 0, 'validUntil'], '2026-10-24T00:00:00.000Z'],
    ['pending-null-review', ['bundle', 'evidence', 0, 'review'], { status: 'pending', reviewerId: null, reviewedAt: null }, true],
    ['observation-edge', ['bundle', 'observations', 0, 'edgeId'], 'missing'],
    ['observation-evidence', ['bundle', 'observations', 0, 'evidenceId'], 'missing'],
    ['measurement-string', ['bundle', 'observations', 0, 'value'], 'wide'],
    ['negative-width', ['bundle', 'observations', 0, 'value'], -1],
    ['negative-uncertainty', ['bundle', 'observations', 0, 'uncertainty'], -1],
    ['unknown-error', ['bundle', 'observations', 0, 'uncertainty'], null, true],
    ['unknown-without-reason', ['bundle', 'observations', 0, 'value'], null],
    ['known-with-reason', ['bundle', 'observations', 0, 'unknownReason'], 'not-surveyed'],
    ['fractional-steps', ['bundle', 'observations', 3, 'value'], 0.5],
    ['fractional-step-error', ['bundle', 'observations', 3, 'uncertainty'], 0.1],
    ['unsafe-step-count', ['bundle', 'observations', 3, 'value'], 2 ** 53],
    ['category-number', ['bundle', 'observations', 4, 'value'], 1],
    ['category-error', ['bundle', 'observations', 4, 'uncertainty'], 0],
    ['transform-local-reference', ['bundle', 'transforms', 0, 'localFrameId'], 'outside'],
    ['transform-enu-reference', ['bundle', 'transforms', 0, 'enuFrameId'], 'plan'],
    ['transform-evidence', ['bundle', 'transforms', 0, 'evidenceId'], 'missing'],
    ['transform-uncertainty', ['bundle', 'transforms', 0, 'uncertaintyMeters'], -1],
    ['transform-control-count', ['bundle', 'transforms', 0, 'controls'], [base.bundle.transforms[0].controls[0]]],
    ['transform-degenerate', ['bundle', 'transforms', 0, 'controls'], [base.bundle.transforms[0].controls[0], base.bundle.transforms[0].controls[0]]],
    ['transform-residual', ['bundle', 'transforms', 0, 'rotationDegrees'], 0, true],
    ['requirement-empty', ['bundle', 'policyRequirements', 0, 'edgeIds'], []],
    ['requirement-duplicate-edge', ['bundle', 'policyRequirements', 0, 'edgeIds'], ['ab', 'ab']],
    ['requirement-unknown-edge', ['bundle', 'policyRequirements', 0, 'edgeIds'], ['missing']],
    ['negative-hard-limit', ['profile', 'hard', 'maxDownhillPercent'], -1],
    ['fractional-hard-steps', ['profile', 'hard', 'maxStepCount'], 0.5],
    ['unsafe-hard-steps', ['profile', 'hard', 'maxStepCount'], 2 ** 53],
    ['empty-categories', ['profile', 'hard', 'allowedSurfaces'], []],
    ['duplicate-categories', ['profile', 'hard', 'allowedDoorOperations'], ['manual', 'manual']],
    ['duplicate-preferences', ['profile', 'preferences', 'avoidKinds'], ['stairs', 'stairs']],
    ['negative-penalty', ['profile', 'preferences', 'penaltyMeters'], -1],
    ['empty-sources', ['profile', 'evidencePolicy', 'acceptedSources'], []],
    ['duplicate-sources', ['profile', 'evidencePolicy', 'acceptedSources'], ['operator', 'operator']],
    ['negative-age', ['profile', 'evidencePolicy', 'maxAgeSeconds'], -1],
    ['negative-transform-budget', ['profile', 'evidencePolicy', 'maxTransformErrorMeters'], -1],
    ['duplicate-policies', ['context', 'policies'], [base.context.policies[0], base.context.policies[0]]],
    ['policy-target', ['context', 'policies', 0, 'closedEdgeIds'], ['missing']],
    ['policy-interval', ['context', 'policies', 0, 'validUntil'], base.context.policies[0].validFrom],
    ['fractional-revision', ['context', 'policies', 0, 'revision'], 0.5],
    ['unsafe-revision', ['context', 'policies', 0, 'revision'], 2 ** 53],
    ['digest-syntax', ['context', 'policies', 0, 'bundleDigest'], 'a'.repeat(63)],
    ['bundle-mismatch', ['context', 'policies', 0, 'bundleDigest'], 'a'.repeat(64), true],
    ['closed', ['context', 'policies', 0, 'closedEdgeIds'], ['ab'], true],
    ['duplicate-issuers', ['context', 'trustedIssuerIds'], ['operator', 'operator']],
    ['untrusted-issuer', ['context', 'trustedIssuerIds'], [], true],
    ['unknown-floor', ['context', 'minimumPolicyRevisions', 'missing'], 0],
    ['unsafe-floor', ['context', 'minimumPolicyRevisions', 'operations'], 2 ** 53],
    ['negative-floor', ['context', 'minimumPolicyRevisions', 'operations'], -1],
    ['missing-floor', ['context', 'minimumPolicyRevisions'], {}, true],
  ];
  for (const [id, path, value, valid] of checks) set(id, path, value, valid);
  for (const [index, timestamp] of ['2026-02-29T00:00:00.000Z', '1900-02-29T00:00:00.000Z', '2026-01-01T24:00:00.000Z', '2026-01-01T00:00:60.000Z', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00.000+00:00'].entries()) {
    set(`invalid-time-${index}`, ['context', 'evaluatedAt'], timestamp);
  }
  for (const timestamp of ['0000-02-29T00:00:00.000Z', '1970-01-01T00:00:00.000Z', '2000-02-29T00:00:00.000Z', '9999-12-31T23:59:59.999Z']) set(`time:${timestamp}`, ['context', 'evaluatedAt'], timestamp, true);
  const cases = definitions.map(item => {
    const { bundle, profile, context } = applyEdits(base, item.edits);
    const valid = !validateBundle(bundle).length && !validateProfile(profile).length && !validateContext(context, bundle).length;
    assert.equal(valid, item.valid, item.id);
    return { ...item, expected: valid ? { digests: inputDigests(bundle, profile, context), assessment: assessPath(bundle, profile, context, 'a', 'b', ['ab']) } : null };
  });
  return { version: '0.1.0-draft.1', base, cases };
}
