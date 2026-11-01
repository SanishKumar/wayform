import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { legacyDirectionCases } from '../scripts/legacy-direction-cases';

test('base directions retain the original 7238 journey corpus', async () => {
  assert.deepEqual(await legacyDirectionCases(), JSON.parse(readFileSync('compat/voicegis/expected/base-directions.json', 'utf8')));
});

test('landmark directions retain the original 7238 journey corpus', async () => {
  assert.deepEqual(await legacyDirectionCases(true), JSON.parse(readFileSync('compat/voicegis/expected/landmark-directions.json', 'utf8')));
});
