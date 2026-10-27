import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { buildEdgeCases } from '../scripts/edge-cases';

test('combined native edge fixtures retain TypeScript decisions, reasons and evidence identities', () => {
  const expected = readFileSync(new URL('../packages/mobility-contract/examples/edges-v0.1.0-draft.1.jsonl', import.meta.url), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.deepEqual(buildEdgeCases(), expected);
});
