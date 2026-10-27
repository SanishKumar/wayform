import assert from 'node:assert/strict';
import { inputDigests } from '../packages/mobility-contract/src/identity';
import { validateBundle, validateContext, validateProfile } from '../packages/mobility-contract/src/validate';
import { addEntrance, fixture } from '../tests/fixtures';

export function buildIdentityCases() {
  const cases: { id: string; data: ReturnType<typeof fixture> }[] = [];
  for (const value of [0, -0, 0.1, 0.000001, 0.0000001, 1e20, 1e21, 1e22, Number.MIN_VALUE, Number.MAX_VALUE, 9007199254740992, 1000000000000000100, 0.30000000000000004]) {
    const data = fixture(); data.bundle.nodes[0].position = [value, -value, 0]; data.profile.evidencePolicy.maxAgeSeconds = value;
    cases.push({ id: `number-${cases.length}`, data });
  }
  const unicode = fixture();
  unicode.bundle.evidence[0].actorId = 'quote" slash/ backslash\\ newline\n tab\t \u0000 \u2028 \u2029 \u{10000} \uE000';
  unicode.bundle.nodes[2].id = '\u{10000}'; unicode.bundle.nodes.push({ ...unicode.bundle.nodes[2], id: '\uE000' });
  cases.push({ id: 'unicode-and-escaping', data: unicode });
  const numericKeys = fixture();
  const ids = ['10', '2', '0', '01', '4294967294', '4294967295', '__proto__', 'constructor', '\u{10000}', '\uE000'];
  numericKeys.bundle.policyRequirements = ids.map(id => ({ id, edgeIds: ['ab'] }));
  numericKeys.context.minimumPolicyRevisions = Object.fromEntries(ids.map((id, index) => [id, index]));
  cases.push({ id: 'numeric-and-unicode-property-keys', data: numericKeys });
  const entrance = fixture(); addEntrance(entrance.bundle);
  entrance.profile.hard.allowedSurfaces = ['paved', 'wood']; entrance.profile.preferences.avoidKinds = ['lift', 'stairs'];
  cases.push({ id: 'unordered-collections', data: entrance });
  return cases.map(({ id, data }) => {
    assert.deepEqual(validateBundle(data.bundle), [], id); assert.deepEqual(validateProfile(data.profile), [], id); assert.deepEqual(validateContext(data.context, data.bundle), [], id);
    return { id, synthetic: true, ...data, expected: inputDigests(data.bundle, data.profile, data.context) };
  });
}
