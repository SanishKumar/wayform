import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { legacyCheckinCases } from '../scripts/legacy-checkin-cases';

test('QR check-ins keep exact payload matching and ECMAScript whitespace rules',async()=>{
  assert.deepEqual(await legacyCheckinCases(),JSON.parse(readFileSync('compat/voicegis/expected/checkins.json','utf8')));
});
