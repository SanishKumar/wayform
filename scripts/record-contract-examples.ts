import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { assessPath } from '../packages/mobility-contract/src/assess';
import { fixture, addEntrance, addObservation, addPolicy } from '../tests/fixtures';

if (!process.argv.includes('--record')) throw new Error('Explicit --record required; expected decisions are review artifacts.');
type Data = ReturnType<typeof fixture>;
const scenarios: { id: string; status: string; change: (data: Data) => void; start?: string; end?: string; edges?: string[] }[] = [
  { id: 'measured-width', status: 'suitable', change: () => {} },
  { id: 'missing-width', status: 'unknown', change: data => { data.bundle.observations = []; } },
  { id: 'missing-slope', status: 'unknown', change: data => { data.profile.hard.maxUphillPercent = 5; } },
  { id: 'too-narrow', status: 'unsuitable', change: data => { data.bundle.observations[0].value = 0.6; } },
  { id: 'conflicting-width', status: 'unknown', change: data => { addObservation(data.bundle, 'clearWidthMeters', 0.6, 0.02); } },
  { id: 'one-way-reversed', status: 'invalid', start: 'b', end: 'a', change: () => {} },
  { id: 'uphill-excluded', status: 'unsuitable', change: data => { data.profile.hard.maxUphillPercent = 5; addObservation(data.bundle, 'slopePercent', 8, 0.2); } },
  { id: 'lift-closed', status: 'unsuitable', change: data => { data.bundle.edges[0].kind = 'lift'; addPolicy(data.bundle, data.context); data.context.policies[0].closedEdgeIds = ['ab']; } },
  { id: 'closure-expired', status: 'unknown', change: data => { addPolicy(data.bundle, data.context); data.context.evaluatedAt = data.context.policies[0].validUntil; } },
  { id: 'entrance-transform-wrong', status: 'unknown', change: data => { addEntrance(data.bundle); data.bundle.transforms[0].rotationDegrees = 0; } },
];
const examples = scenarios.map(scenario => {
  const data = fixture(); scenario.change(data);
  const request = { startId: scenario.start ?? 'a', destinationId: scenario.end ?? 'b', edgeIds: scenario.edges ?? ['ab'] };
  const expected = assessPath(data.bundle, data.profile, data.context, request.startId, request.destinationId, request.edgeIds);
  assert.equal(expected.status, scenario.status, scenario.id);
  return { id: scenario.id, ...data, request, expected };
});
const directory = new URL('../packages/mobility-contract/examples/', import.meta.url);
mkdirSync(directory, { recursive: true });
writeFileSync(new URL('decisions-v0.1.0-draft.1.json', directory), JSON.stringify({ synthetic: true, purpose: 'Fictional measurements and identities for language-independent contract conformance.', examples }, null, 2) + '\n');
console.log(`Recorded ${examples.length} synthetic contract examples.`);
