import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildLegacySearchCases } from '../scripts/legacy-search-cases';

test('legacy search fixtures retain the original queue, flags and numeric ordering', () => {
  const recorded = readFileSync(new URL('../compat/voicegis/expected/search-kernel.jsonl', import.meta.url), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.deepEqual(buildLegacySearchCases(), recorded);
  assert.notDeepEqual(recorded[0].expected.pathIds, recorded[1].expected.pathIds);
});
