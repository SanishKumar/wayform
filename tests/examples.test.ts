import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { assessPath } from '../packages/mobility-contract/src/assess';
import type { EvaluationContext, MobilityBundle, MobilityProfile, PathAssessment } from '../packages/mobility-contract/src/types';

test('portable contract examples retain their recorded decisions and input identities', () => {
  const document = JSON.parse(readFileSync(new URL('../packages/mobility-contract/examples/decisions-v0.1.0-draft.1.json', import.meta.url), 'utf8')) as {
    synthetic: boolean;
    examples: { id: string; bundle: MobilityBundle; profile: MobilityProfile; context: EvaluationContext; request: { startId: string; destinationId: string; edgeIds: string[] }; expected: PathAssessment }[];
  };
  assert.equal(document.synthetic, true);
  assert.equal(document.examples.length, 10);
  for (const item of document.examples) assert.deepEqual(assessPath(item.bundle, item.profile, item.context, item.request.startId, item.request.destinationId, item.request.edgeIds), item.expected, item.id);
});
