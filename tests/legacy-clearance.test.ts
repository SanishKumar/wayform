import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { legacyClearanceCorpus, legacyClearanceNegativeCases } from '../scripts/legacy-clearance-cases';

test('graphic clearance retains the pinned journeys and rejects inconsistent authored geometry', async () => {
  assert.deepEqual({ corpus: await legacyClearanceCorpus(), negative: await legacyClearanceNegativeCases() }, JSON.parse(readFileSync('compat/voicegis/expected/clearance.json', 'utf8')));
});
