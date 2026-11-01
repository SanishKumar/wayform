import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { legacyRuntimeCases } from '../scripts/legacy-runtime-cases';

test('visitor metadata keeps the pinned destination inventory and runtime projection', async () => {
  assert.deepEqual(await legacyRuntimeCases(), JSON.parse(readFileSync('compat/voicegis/expected/runtime.json', 'utf8')));
});
