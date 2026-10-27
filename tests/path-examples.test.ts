import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { buildPathCases } from '../scripts/path-cases';

test('native path receipts retain supplied-path contract behavior', () => {
  const recorded = readFileSync(new URL('../packages/mobility-contract/examples/paths-v0.1.0-draft.1.jsonl', import.meta.url), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  assert.deepEqual(buildPathCases(), recorded);
});
